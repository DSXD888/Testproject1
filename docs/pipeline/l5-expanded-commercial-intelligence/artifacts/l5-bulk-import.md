# L5 Expanded Commercial Intelligence — Bead Bulk Import

> **Authority:** This document is the bead-creation manifest for Layer 5. Each section below is a self-contained bead definition embedding verbatim spec text, TDD test IDs, and concrete file paths. The L5 spec, TDD, and bootstrap remain authoritative.
>
> **Source documents:**
> - Spec (SOT #1): `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` (130 L5-FR-* requirements)
> - TDD (SOT #2): `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md` (106 checks: 8 static gates + 98 stable L5-T-* assertion IDs across 57 runnable functions)
> - Strategy (SOT #3, ordering only): `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md`
> - Bootstrap: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md`
>
> **Bead totals:** 1 sister implementation epic + 6 phase sub-epics + 54 task beads = 61 beads.
>
> **Bead CLI:** `br` (NOT `bd`). Sync: `bin/br-sync` then manual `git add .beads/ && git commit && git push`.
>
> **Parent epic:** `testproject1-hn05` (Epic: L5 Implementation — Expanded Commercial Intelligence), sister to the L5 documentation epic `testproject1-4gh` per Decision 3 recorded 2026-05-08.
>
> **Coverage assertion:** every L5-FR-X.Y.Z appears in at least one bead's description (verified 130/130). Every L5-T-* test ID is owned by exactly one task bead's `Tests to Pass` list (review-gate beads aggregate phase coverage but do not double-own test IDs). Trace artifact: `l5-bead-alias-map.json`.

---

# Root Sister Implementation Epic

## Epic: L5 Implementation — Expanded Commercial Intelligence

- id: `testproject1-hn05`
- type: epic
- priority: P0
- labels: implementation,l5,layer-5
- parent: (top-level)

### Description

# Epic: L5 Implementation — Expanded Commercial Intelligence

## SOURCE OF TRUTH WARNING

This epic threads all Phase 1-6 implementation beads for L5. Bead bodies are sourced verbatim from:

- **Spec (SOT #1)**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`
- **TDD (SOT #2)**: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md`
- **Strategy (SOT #3, ordering only)**: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md`
- **Bootstrap**: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md`

The spec records 130 unique L5-FR-* IDs (1.1.1 through 3.10.2). The TDD declares 98 stable L5-T-* assertion IDs across 5 package test files plus 3 daemon integration targets. Every L5-FR must map to at least one bead; every L5-T must be owned by exactly one bead. This epic is the sister to the L5 documentation epic `testproject1-4gh` (per Decision 3 recorded 2026-05-08; `4gh` children .1-.4 are the docs-authoring work, this epic is the implementation work).

## Phase Structure (from strategy)

| Phase | Name | Task Beads | Primary Gate |
|-------|------|------------|--------------|
| 1 | Foundation: Schema And Daemon Glue | 8 | CI-L5-001..003,005..007; L5-T-customs_classification-006; L5-T-measure_interaction-018; L5-T-fx_engine-009; L5-T-freight_engine-012,014..016 |
| 2 | FX Engine | 6 | L5-T-fx_engine-001..012; CI-L5-004 |
| 3 | Freight Engine | 7 | L5-T-freight_engine-001..008,015,016 |
| 4 | Customs Engine | 13 | L5-T-hmrc_fpo-*; L5-T-customs_classification-001..024,033,034,036; L5-T-measure_interaction-001..020 |
| 5 | Marketplace Skills + Validator | 9 | L5-T-freight_engine-009..014,016,018; L5-T-customs_classification-008,011 |
| 6 | Gold-Set / CBAM / Integration | 11 | Full Stage Gate; CI-L5-001..008; L5-T-customs_classification-009,010,012,025..032,035,036; L5-T-freight_engine-017; L5-T-measure_interaction-020 |
| Total | — | **54 task beads** | Plus 6 phase sub-epics + this root |

## Operator Decisions (2026-05-08)

1. **product_commodity_codes ownership** — Option A: new SQLite migration `migrations/0013_l5_product_commodity_codes.sql`.
2. **PCS Scotland scope** — IN-SCOPE: full skill directory + daemon registry entry in Phase 5.
3. **Implementation epic structure** — Option (b) sister epic (this epic). Layer-doc epic `testproject1-4gh` keeps its `.1`-`.4` children.
4. **Gold-set categories (DoD #12)** — fasteners + packaging-cartons + examination-gloves.

## Exit Criteria (DoD walkthrough)

L5 closes only when:

- All six phase review gates are green.
- The mandatory Stage Validation Test Gate passes: cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test -p daemon --bin daemon && for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done && bun run build && bun run test.
- All CI-L5-001..008 static gates pass.
- All twelve DoD rows (L5-FR-3.1.1..L5-FR-3.1.12) pass in one walkthrough.
- L5→L6 readiness fixture proves every blocker condition (CLASSIFICATION_AMBIGUOUS, TARIFF_BRANCH_DIVERGENCE, required SUPPLEMENTARY_UNIT_MISSING, decision_grade=broker_required, expired classification_fresh_until) can be produced and blocks bid readiness.

## Bead Discipline

- Bead prefix: `l5`. CLI: `br` (NEVER `bd`). Sync via `bin/br-sync` (NEVER raw `br sync`).
- Every bead claim/complete uses `bin/br-claim <bead-id>` / `bin/br-complete <bead-id>` per AGENTS.md Rule 2.
- Every bead embeds spec L5-FR text + TDD L5-T test IDs verbatim. Phases are execution order, not importance tiers.

## Full L5-FR Coverage Cross-Reference (130 IDs)

Every L5-FR-X.Y.Z requirement in the spec is covered by at least one bead. Beads list FR IDs by topic; this table is the exhaustive map for any FR that might not appear verbatim in its primary bead body.

### §1 Overview & Customs Classification Engine

- **L5-FR-1.1.1** (L5 expands screening to decision-grade) → `hn05.4.13` (customs-scope-preservation-gate) — closes the integrated customs surface
- **L5-FR-1.2.1** → `hn05.6.3/6.4/6.5` gold-set corpora
- **L5-FR-1.2.2** (≥2 supplier sources per category) → `hn05.5.8` (marketplace-fixture-promotion-gate)
- **L5-FR-1.2.3** (full customs classification) → `hn05.4.13` (customs-scope-preservation-gate)
- **L5-FR-1.2.4** (dual FX + dual freight) → `hn05.2.6` (fx gate) + `hn05.3.7` (freight gate)
- **L5-FR-1.2.5** (per-category gold-set required) → `hn05.6.3/6.4/6.5`
- **L5-FR-1.3.1** → `hn05.4.12` (redaction-sentinel-design)
- **L5-FR-1.3.2** → `hn05.1.1` (sqlite-product-commodity-codes)
- **L5-FR-1.3.3** (supplier_quotes activation) → `hn05.5.8` (promotion gate) + `hn05.5.2/3/4/5`
- **L5-FR-1.4.1..1.4.5** → `hn05.4.1` (fpo-taric), `hn05.4.2` (description-quality), `hn05.4.3` (multi-family)
- **L5-FR-1.5.1..1.5.5** → `hn05.4.5` (uk10-resolution)
- **L5-FR-1.6.1..1.6.2** → `hn05.4.6` (exporter-origin)
- **L5-FR-1.7.1..1.7.4** → `hn05.4.7` (measure-scanner)
- **L5-FR-1.8.1..1.8.4** → `hn05.4.7` (measure-scanner)
- **L5-FR-1.9.1..1.9.2** → `hn05.4.8` (vat-additional-code)
- **L5-FR-1.10.1..1.10.2** → `hn05.4.8` (vat-additional-code)
- **L5-FR-1.11.1..1.11.4** → `hn05.4.9` (classification-freshness), `hn05.1.5` (cron-definitions)
- **L5-FR-1.12.1..1.12.4** → `hn05.4.10` (conservative-defaults-evidence-leverage)
- **L5-FR-1.13.1..1.13.2** → `hn05.4.11` (divergence-readiness-producers)
- **L5-FR-1.14.1..1.14.3** → `hn05.6.6` (cbam-placeholder)
- **L5-FR-1.15.1** → `hn05.5.2/3/4/5` (alibaba/1688/amazon/ebay maturation)
- **L5-FR-1.15.2** → `hn05.5.6` (pcs-scotland-skill)
- **L5-FR-1.16.1..1.16.2** → 19-file package surface across `hn05.2.*` + `hn05.3.*` + `hn05.4.*` + `hn05.6.1` + `hn05.6.2`

### §2 FX, Freight & Multi-Source Marketplace Search

- **L5-FR-2.1.1..2.1.4** → `hn05.2.1` through `hn05.2.6` (Phase 2 FX engine)
- **L5-FR-2.2.1..2.2.4** → `hn05.3.1` through `hn05.3.7` (Phase 3 freight)
- **L5-FR-2.3.1** → `hn05.5.2` (alibaba)
- **L5-FR-2.3.2** → `hn05.5.3` (1688)
- **L5-FR-2.3.3** → `hn05.5.4` (amazon)
- **L5-FR-2.3.4** → `hn05.1.4` (source-registry-shape) + `hn05.5.4` (amazon) + `hn05.5.6` (pcs)
- **L5-FR-2.3.5** → `hn05.5.1` (validator-overlay-contract) + `hn05.5.8` (promotion gate)
- **L5-FR-2.3.6** → `hn05.5.2` (alibaba)
- **L5-FR-2.4.1** → `hn05.4.13` (customs gate) + `hn05.6.9` (mainrs-delegation-cutover) — full column population proven by `L5-T-customs_classification-036`
- **L5-FR-2.4.2** → `hn05.4.4` (tariff-graph-junction-writes)
- **L5-FR-2.4.3** → `hn05.5.8` (marketplace-fixture-promotion-gate)

### §3 Testing, DoD, Risks & Open Questions

- **L5-FR-3.1.1** (DoD 1 Alibaba/1688) → `hn05.5.2` + `hn05.5.3`
- **L5-FR-3.1.2** (DoD 2 Amazon) → `hn05.5.4` (amazon-maturation)
- **L5-FR-3.1.3** (DoD 3 PCS) → `hn05.5.6` (pcs-scotland-skill)
- **L5-FR-3.1.4** (DoD 4 FPO investigation) → `hn05.4.1` (hmrc-fpo-taric-discovery)
- **L5-FR-3.1.5** (DoD 5 multi-source recall) → `hn05.4.3` (hmrc-multi-family-expansion)
- **L5-FR-3.1.6** (DoD 6 UK10 via HMRC) → `hn05.4.5` (uk10-resolution)
- **L5-FR-3.1.7** (DoD 7 measure scanner) → `hn05.4.7` (measure-scanner-interactions)
- **L5-FR-3.1.8** (DoD 8 HMRC customs FX) → `hn05.2.2` (hmrc-customs-fx-parser)
- **L5-FR-3.1.9** (DoD 9 Frankfurter) → `hn05.2.3` (frankfurter-commercial-fx)
- **L5-FR-3.1.10** (DoD 10 Freightos+SeaRates) → `hn05.3.3` + `hn05.3.4`
- **L5-FR-3.1.11** (DoD 11 decision-grade landed cost) → `hn05.6.10` (default-on-live-demo) + `hn05.6.11` (final-dod-gate)
- **L5-FR-3.1.12** (DoD 12 gold-set) → `hn05.6.3/4/5`
- **L5-FR-3.2.1** (5 test files cumulative 57) → `hn05.6.11` (final-dod-readiness-gate) — package inventory proof
- **L5-FR-3.2.2** (Stage Validation Test Gate) → `hn05.6.11` (final-dod-readiness-gate)
- **L5-FR-3.3.1** (multi-family expansion scenario) → `hn05.4.5` (uk10-resolution)
- **L5-FR-3.3.2** (steel trade remedy scenario) → `hn05.4.7` (measure-scanner) + `hn05.6.3` (fasteners corpus)
- **L5-FR-3.3.3** (compound duty scenario) → `hn05.4.7` (measure-scanner)
- **L5-FR-3.3.4** (supplementary unit missing scenario) → `hn05.4.7` (measure-scanner)
- **L5-FR-3.3.5** (named exporter aluminium scenario) → `hn05.4.6` (exporter-origin-resolution)
- **L5-FR-3.3.6** (additional-code unknown scenario) → `hn05.4.7` (measure-scanner)
- **L5-FR-3.3.7** (GIR 3 set scenario) → `hn05.4.7` (measure-scanner)
- **L5-FR-3.3.8** (FX monthly rollover scenario) → `hn05.2.2` (hmrc-customs-fx-parser)
- **L5-FR-3.3.9** (Alibaba Chinese retry scenario) → `hn05.5.2` (alibaba)
- **L5-FR-3.3.10** (category whitelist scenario) → `hn05.4.13` (customs-scope-gate) + `hn05.6.11`
- **L5-FR-3.3.11** (tariff structure mid-cycle change scenario) → `hn05.4.9` (classification-freshness)
- **L5-FR-3.3.12** (conservative-to-actual uplift scenario) → `hn05.4.10` (conservative-defaults-evidence-leverage)
- **L5-FR-3.3.13** (1688 Chinese-only scenario) → `hn05.5.3` (1688)
- **L5-FR-3.3.14** (Amazon browser-first scenario) → `hn05.5.4` (amazon)
- **L5-FR-3.3.15** (PCS OCDS scenario) → `hn05.5.6` (pcs)
- **L5-FR-3.3.16** (FX basis delta scenario) → `hn05.2.4` (fx-basis-recalculation)
- **L5-FR-3.3.17** (Freight provider disagreement scenario) → `hn05.3.5` (dual-provider-disagreement)
- **L5-FR-3.3.18** (gold-set category promotion scenario) → `hn05.6.3/4/5`
- **L5-FR-3.3.19** (CBAM 2027-crossing scenario) → `hn05.6.6` (cbam-placeholder)
- **L5-FR-3.4.1..3.4.3** (evidence-tiered accuracy) → `hn05.6.1` (accuracy-harness-module) + `hn05.6.3/4/5`
- **L5-FR-3.5.1..3.5.10** (customs error vocabulary) → `hn05.1.2` (error-vocabulary-enum) + `hn05.1.3` (error-exhaustive-consumers) + emitting beads
- **L5-FR-3.6.1..3.6.2** (open questions runtime behavior) → distributed: `hn05.3.6` (freight boundary), `hn05.4.9` (freshness), `hn05.5.3` (1688), `hn05.6.6` (CBAM)
- **L5-FR-3.7.1** (FPO risk) → `hn05.4.1` (fpo-taric-discovery)
- **L5-FR-3.7.2** (anti-bot risk) → `hn05.5.2` + `hn05.5.7` (source-health-auth-binding)
- **L5-FR-3.7.3** (customs classification wrong risk) → `hn05.6.1` (accuracy-harness)
- **L5-FR-3.7.4** (freight source variability risk) → `hn05.3.4` (searates-adapter) + `hn05.3.7` (freight gate)
- **L5-FR-3.7.5** (tariff cache freshness gap risk) → `hn05.4.9` (classification-freshness)
- **L5-FR-3.7.6** (cross-language matching corruption risk) → `hn05.5.2` (alibaba — back-translation check) + `hn05.5.3` (1688)
- **L5-FR-3.7.7** (worker/validator version drift risk) → `hn05.5.1` (validator-overlay-contract)
- **L5-FR-3.7.8** (CBAM rate publication delay risk) → `hn05.6.6` (cbam-placeholder)
- **L5-FR-3.8.1..3.8.2** (L6 bid-readiness gate) → `hn05.4.11` (divergence-readiness-producers) + `hn05.6.11` (final-dod-gate)
- **L5-FR-3.9.1** (L5 depends on L0-L4) → `hn05.1.6` (env-bootstrap-preflight)
- **L5-FR-3.9.2** (operator-built prerequisites gate) → `hn05.3.1` (freightos-prerequisite-check) + `hn05.4.1` (taric)
- **L5-FR-3.9.3** (br vs deprecated bd discipline) → `hn05.5.5` (ebay-baseline-binding — per strategy mapping) + every bead's Coordination Protocol block
- **L5-FR-3.10.1..3.10.2** (ADR-006, ADR-001) → distributed across `hn05.2.4` + `hn05.6.11`

## Bead Count Reconciliation

- 1 root sister epic (`testproject1-hn05`) + 6 phase sub-epics (`hn05.1..hn05.6`) + 54 task beads (`hn05.N.M`) = **61 l5-labeled beads**.
- Strategy projected 54 task beads + 6 phase epics; this bead set matches exactly.
- Sister-epic structure (Decision 3 option b) keeps the L5 documentation epic `testproject1-4gh` (with its `.1-.4` children) separate from implementation.

## Success Criteria

- All 6 phase review gates green (each phase's preservation gate bead closed via bin/br-complete).
- Mandatory Stage Validation Test Gate passes: `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test -p daemon --bin daemon && for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done && bun run build && bun run test`.
- All CI-L5-001..008 static gates pass.
- All 12 DoD rows (L5-FR-3.1.1..L5-FR-3.1.12) verified in one walkthrough recorded under `docs/pipeline/l5-expanded-commercial-intelligence/live-demo/`.
- L5→L6 readiness fixture produces every blocker: CLASSIFICATION_AMBIGUOUS, TARIFF_BRANCH_DIVERGENCE, required SUPPLEMENTARY_UNIT_MISSING, decision_grade=broker_required, expired classification_fresh_until — and blocks bid readiness.
- 19 package files present in fixed 6/4/5/3/1 distribution; 5 package test files; 3 daemon integration targets.
- 3 gold-set corpora (fasteners + packaging-cartons + examination-gloves) score `production_gate: eligible`.
- 100% L5-FR coverage (130/130) + 100% L5-T coverage (98/98).

---

# Phase Sub-Epics

## L5 Phase 1: Foundation — Schema And Daemon Glue

- id: `testproject1-hn05.1`
- type: epic
- priority: P0
- labels: implementation,l5,l5-phase-1,layer-5
- parent: `testproject1-hn05`

### Description

# L5 Phase 1: Foundation — Schema And Daemon Glue

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — §1.3.2, §1.11.2, §1.15.2, §2.3.4, §2.4.2, §3.5.1-3.5.10, §3.9.1-3.9.3, §3.10.2 (PRIMARY)
- **TDD**: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md` — §7 (CI-L5-001..007), `L5-T-customs_classification-006`, `L5-T-measure_interaction-018`, `L5-T-fx_engine-009`, `L5-T-freight_engine-012,014..016` (PRIMARY)
- **Strategy**: `...l5-expanded-commercial-intelligence-strategy.md` — Phase 1 (ordering)
- **Bootstrap**: `...l5-expanded-commercial-intelligence-bootstrap.md` — §3 (env), §6.1 (package surface), §8 (cron seeding)

## Overview

Foundation phase wires the local SQLite `product_commodity_codes` junction, extends the daemon `ErrorCode` enum + matcher + exhaustive consumers with 9 new L5 variants (reusing existing `FpoAccessUnavailable`), registers 4-5 new source-registry entries (Amazon, PCS, plus marketplace/CDP source-health bindings), seeds 4 new cron definitions, and updates env-var/preflight bootstrap. All later phases compile against the foundation set here.

## Phase 1 Task Beads (8)

1. `l5-p1-sqlite-product-commodity-codes` — migration `migrations/0013_l5_product_commodity_codes.sql`
2. `l5-p1-error-vocabulary-enum` — `ErrorCode` enum + `as_str()` matcher
3. `l5-p1-error-exhaustive-consumers` — clippy-surfaced match sites
4. `l5-p1-source-registry-shape` — Amazon, PCS, marketplace source-health registry entries
5. `l5-p1-cron-definitions` — 4 cron entries in `scheduler/cron.rs`
6. `l5-p1-env-bootstrap-preflight` — env vars + preflight script
7. `l5-p1-daemon-test-target-setup` — three daemon test files entry-point setup
8. `l5-p1-scope-preservation-gate` — phase review gate

## Gate Criteria (from strategy §Phase 1)

- `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p daemon --bin daemon` all pass for Phase 1 scope.
- `product_commodity_codes` local table applies from scratch; set-replacement is test-owned.
- L5 error vocabulary contains the 9 new daemon variants, reuses existing `FpoAccessUnavailable`, updates `parse_error_code`, closes all exhaustive propagation sites.
- 4 L5 cron definitions visible through scheduler definition surface.
- No paid API env var or procurement instruction appears in L5 bootstrap/strategy.

## Roll Back If (from strategy)

- Local `product_commodity_codes` cannot mirror L4 projection without changing L4 sync semantics.
- L5 error handling requires generic string fallbacks instead of typed daemon error vocabulary.
- Cron entries cannot fit the existing `CronJobDefinition` pattern.

## Success Criteria

- `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p daemon --bin daemon` all pass for Phase 1 scope.
- `migrations/0013_l5_product_commodity_codes.sql` (or next-free NNNN) applies cleanly from fresh DB; set-replacement test passes.
- L5 ErrorCode enum contains the 9 new variants + reuses existing FpoAccessUnavailable; `parse_error_code` + `as_str()` matchers updated; exhaustive call sites closed.
- 4 new CronJobDefinition entries visible in scheduler surface.
- New RegisteredSource entries (pcs, amazon production binding, l5-marketplace-commercial-intelligence) present in `crates/daemon/src/sources/registry.rs`.
- No paid API env var or procurement instruction in any L5 bootstrap/strategy doc.
- All 8 P1 task beads closed via `bin/br-complete`.

---

## L5 Phase 2: FX Engine

- id: `testproject1-hn05.2`
- type: epic
- priority: P0
- labels: implementation,l5,l5-phase-2,layer-5
- parent: `testproject1-hn05`

### Description

# L5 Phase 2: FX Engine

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — §1.2.4, §2.1.1-2.1.4, §3.1.8, §3.1.9, §3.3.8, §3.3.16, §3.5.9, §3.9.1, §3.10.1, §3.10.2 (PRIMARY)
- **TDD**: `...l5-expanded-commercial-intelligence-tdd.md` — §8.4 (`L5-T-fx_engine-001..012`), CI-L5-004 (PRIMARY)
- **Strategy**: `...l5-expanded-commercial-intelligence-strategy.md` — Phase 2 (ordering)

## Overview

Dual FX engine: HMRC monthly customs FX (legal authority) + Frankfurter commercial spot (margin estimation). Computes basis delta, detects monthly rollover and historical corrections, emits `FX_RATE_MISSING` when no valid customs snapshot exists. 4 package files under `packages/intelligence/src/fx/` plus extension of `skills/landed-cost/simple-fx-lookup/`.

## Phase 2 Task Beads (6)

1. `l5-p2-fx-types-and-cassettes` — `fx-types.ts` + cassettes scaffolding
2. `l5-p2-hmrc-customs-fx-parser` — `hmrc-exchange-rates.ts`
3. `l5-p2-frankfurter-commercial-fx` — `frankfurter-commercial-rates.ts`
4. `l5-p2-fx-basis-recalculation` — `fx-engine.ts` (basis delta, recalc triggers)
5. `l5-p2-simple-fx-skill-extension` — extend `skills/landed-cost/simple-fx-lookup/`
6. `l5-p2-fx-cron-persistence-gate` — review gate including HMRC monthly rollover cron

## Gate Criteria (from strategy §Phase 2)

- `bun run build` and `bun run test` include `fx-engine.test.ts`.
- `L5-T-fx_engine-001..012` pass.
- HMRC monthly rollover cron dispatches recalculation work in daemon test surface.
- Fixture row persists `customs_fx_rate`, `commercial_fx_rate`, `fx_basis_delta_bps`, `fx_rate_checked_at`.
- `FX_RATE_MISSING` behavior is typed and does not fabricate a customs FX snapshot.

## Roll Back If

- HMRC monthly source cannot be parsed deterministically from public/free data or checked-in cassettes.
- Frankfurter integration reintroduces a paid commercial FX dependency.
- FX basis recalculation needs hidden mutable state outside the package/daemon persistence contract.

## Success Criteria

- `bun run build && bun run test` includes `fx-engine.test.ts` and passes `L5-T-fx_engine-001..012` (12 IDs).
- HMRC monthly rollover cron dispatches recalculation work in daemon test surface.
- Fixture row persists `customs_fx_rate`, `commercial_fx_rate`, `fx_basis_delta_bps`, `fx_rate_checked_at` on `landed_cost_estimates`.
- `FX_RATE_MISSING` emitted only when no valid customs snapshot exists; never fabricated.
- `packages/intelligence/src/fx/` contains exactly 4 files: `hmrc-exchange-rates.ts`, `frankfurter-commercial-rates.ts`, `fx-engine.ts`, `fx-types.ts`.
- `skills/landed-cost/simple-fx-lookup/` extended to dual-rate; full 6-file skill directory standard.
- All 6 P2 task beads closed.

---

## L5 Phase 3: Freight Engine

- id: `testproject1-hn05.3`
- type: epic
- priority: P0
- labels: implementation,l5,l5-phase-3,layer-5
- parent: `testproject1-hn05`

### Description

# L5 Phase 3: Freight Engine

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — §2.2.1-2.2.4, §3.1.10, §3.3.17, §3.5.10, §3.6.1, §3.6.2, §3.7.4, §3.9.2 (PRIMARY)
- **TDD**: `...l5-expanded-commercial-intelligence-tdd.md` — §8.5 (`L5-T-freight_engine-001..008,015,016`) (PRIMARY)
- **Strategy**: `...l5-expanded-commercial-intelligence-strategy.md` — Phase 3 (ordering)

## Overview

Dual-source freight engine: Freightos (operator-built CDP skill) + SeaRates (existing CDP skill). Mode selector with hard locks (hazmat, battery, reefer, oversized, fragile). Provider disagreement triggers `FREIGHT_MODE_LOW_CONFIDENCE`; dual failure triggers `FREIGHT_ESTIMATE_FAILED`. 5 package files under `packages/intelligence/src/logistics/`. Phase 3 beads CANNOT claim until `skills/landed-cost/freightos-freight-estimate/{SKILL.md, result.schema.json, checkpoint.schema.json}` all exist.

## Phase 3 Task Beads (7)

1. `l5-p3-freightos-prerequisite-check` — verify operator-built Freightos skill files
2. `l5-p3-logistics-types-mode-selector` — `freight-types.ts` + `mode-selector.ts`
3. `l5-p3-freightos-adapter` — `freightos-skill-adapter.ts`
4. `l5-p3-searates-adapter` — `searates-skill-adapter.ts`
5. `l5-p3-dual-provider-disagreement` — disagreement detection + low-confidence path
6. `l5-p3-freight-boundary-defaults` — `freight-engine.ts` + Incoterm/packing boundaries
7. `l5-p3-freight-source-health-gate` — source-health cron binding + phase review

## Gate Criteria (from strategy §Phase 3)

- `freight-engine.test.ts` covers `L5-T-freight_engine-001..008,015,016`.
- Dual-source disagreement emits low-confidence warning path under fixture data.
- Dual-provider failure emits `FREIGHT_ESTIMATE_FAILED` and preserves manual-estimate requirement.
- Missing logistics assumptions follow §3.6 present-tense runtime behavior.
- Freightos and SeaRates stay CDP/browser skill paths with no paid API fallback.

## Roll Back If

- Freightos or SeaRates can only be integrated through paid API procurement.
- Provider failure handling cannot distinguish single-provider degradation from dual-provider blockers.
- Logistics defaults cannot be represented without relaxing conservative landed-cost assumptions.

## Success Criteria

- `freight-engine.test.ts` covers `L5-T-freight_engine-001..008,015,016` (10 IDs).
- Dual-source disagreement emits `FREIGHT_MODE_LOW_CONFIDENCE` under fixture data; conservative (higher base) selected.
- Dual-provider failure emits `FREIGHT_ESTIMATE_FAILED` and preserves manual-estimate requirement.
- Missing logistics assumptions follow §3.6 present-tense runtime behavior (highest-cost feasible boundary).
- Freightos + SeaRates remain CDP/browser skill paths; no paid API.
- `packages/intelligence/src/logistics/` contains exactly 5 files: `mode-selector.ts`, `freightos-skill-adapter.ts`, `searates-skill-adapter.ts`, `freight-engine.ts`, `freight-types.ts`.
- Source-health canary cron covers Freightos/SeaRates/TARIC readiness.
- All 7 P3 task beads closed.

---

## L5 Phase 4: Customs Engine

- id: `testproject1-hn05.4`
- type: epic
- priority: P0
- labels: implementation,l5,l5-phase-4,layer-5
- parent: `testproject1-hn05`

### Description

# L5 Phase 4: Customs Engine

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — §1.1.1, §1.2.3, §1.3.1, §1.4-1.13, §2.4.1, §3.1.4-3.1.7, §3.3.1-3.3.7, §3.3.10-3.3.12, §3.4.2, §3.5.1-3.5.8, §3.6.1-3.6.2, §3.7.1,3.7.3,3.7.5,3.7.6, §3.8.1, §3.8.2 (PRIMARY)
- **TDD**: `...l5-expanded-commercial-intelligence-tdd.md` — §8.1 (`L5-T-hmrc_fpo-*`), §8.2 (`L5-T-customs_classification-001..024,033,034,036`), §8.3 (`L5-T-measure_interaction-001..020`) (PRIMARY)
- **Strategy**: `...l5-expanded-commercial-intelligence-strategy.md` — Phase 4 (ordering)

## Overview

Customs engine: multi-source recall (5 source families), multi-family expansion to UK10, exporter/origin resolution, measure interaction scanner (additive/substitutive/conditional/documentary_only/unknown), supplementary unit handling, VAT determination, valuation mode, classification freshness (4 independent vectors). 6 package files under `packages/intelligence/src/customs/`. Phase 4 TARIC cross-check beads cannot claim until `skills/landed-cost/taric-cross-check/{SKILL.md, result.schema.json, checkpoint.schema.json}` all exist.

## Phase 4 Task Beads (13)

1. `l5-p4-hmrc-fpo-taric-discovery` — FPO endpoint investigation + TARIC adapter
2. `l5-p4-description-quality-recall` — `description-quality.ts` + initial recall scaffolding
3. `l5-p4-hmrc-multi-family-expansion` — `multi-source-recall.ts` with 5 source families
4. `l5-p4-tariff-graph-junction-writes` — local tariff graph cache + `product_commodity_codes` set-replace
5. `l5-p4-uk10-resolution-branching` — `ten-digit-resolution.ts` (UK10 shortlisting)
6. `l5-p4-exporter-origin-resolution` — `exporter-resolution.ts`
7. `l5-p4-measure-scanner-interactions` — `measure-scanner.ts` (interaction families + compound duty + supplementary units)
8. `l5-p4-vat-additional-code-valuation` — VAT rate + additional-code + valuation mode
9. `l5-p4-classification-freshness` — `classification-freshness.ts` (4 freshness vectors)
10. `l5-p4-conservative-defaults-evidence-leverage` — conservative defaults + differential margin math
11. `l5-p4-divergence-readiness-producers` — TARIFF_BRANCH_DIVERGENCE + CLASSIFICATION_AMBIGUOUS persistence (L6 gate)
12. `l5-p4-redaction-sentinel-design` — automation-sensitive field sentinel taxonomy
13. `l5-p4-customs-scope-preservation-gate` — phase review gate

## Gate Criteria (from strategy §Phase 4)

- All `hmrc-fpo.test.ts`, `customs-classification.test.ts`, and `measure-interaction.test.ts` Phase 4 IDs pass.
- Multi-family expansion exercises at least 3 plausible HS6 families before UK10 shortlisting.
- `product_commodity_codes` writes use candidate-level set replacement under one `stage_run_id`.
- Conservative defaults and all L5 customs blockers emit through typed error paths.
- L5→L6 readiness producers exist for classification ambiguity, tariff branch divergence, required supplementary-unit missing, broker-required decision grade, expired classification freshness.
- Redaction/sync sentinel design is ready for Phase 6 projection-boundary implementation.

## Roll Back If

- Classification requires greedy descent to pass tests.
- TARIC disagreement overwrites HMRC UK national measure data.
- Branch divergence is averaged into one actionable estimate.
- L6 readiness blockers cannot be produced from persisted L5 landed-cost evidence.

## Success Criteria

- All `hmrc-fpo.test.ts` (12 IDs), `customs-classification.test.ts` (Phase 4 owns 001..024, 033, 034, 036 — 28 IDs), and `measure-interaction.test.ts` (20 IDs) Phase 4 test IDs pass.
- Multi-family expansion exercises ≥3 plausible HS6 families before UK10 shortlisting; no greedy descent.
- `product_commodity_codes` writes use candidate-level set replacement under one `stage_run_id`.
- All 9 typed L5 customs blockers emit through daemon error vocabulary (Phase 1 enum extension).
- L5→L6 readiness producers all exist: CLASSIFICATION_AMBIGUOUS, TARIFF_BRANCH_DIVERGENCE, required SUPPLEMENTARY_UNIT_MISSING, broker-required decision grade, expired classification_fresh_until.
- TARIC disagreement recorded as evidence divergence; never overrules HMRC UK national measures.
- Redaction sentinel taxonomy ready for Phase 6 projection boundary.
- `packages/intelligence/src/customs/` contains exactly 6 files: `description-quality.ts`, `multi-source-recall.ts`, `ten-digit-resolution.ts`, `measure-scanner.ts`, `exporter-resolution.ts`, `classification-freshness.ts`.
- `crates/daemon/tests/customs_classification.rs` passes.
- All 13 P4 task beads closed.

---

## L5 Phase 5: Marketplace Skills And Validator Overlay

- id: `testproject1-hn05.5`
- type: epic
- priority: P0
- labels: implementation,l5,l5-phase-5,layer-5
- parent: `testproject1-hn05`

### Description

# L5 Phase 5: Marketplace Skills And Validator Overlay

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — §1.2.2, §1.3.3, §1.15.1, §1.15.2, §2.3.1-2.3.6, §2.4.3, §3.1.1-3.1.3, §3.3.9, §3.3.13-3.3.15, §3.6.1-3.6.2, §3.7.2, §3.7.6, §3.7.7 (PRIMARY)
- **TDD**: `...l5-expanded-commercial-intelligence-tdd.md` — §8.5 (`L5-T-freight_engine-009..014,016,018`), §8.2 (`L5-T-customs_classification-008,011`) (PRIMARY)
- **Strategy**: `...l5-expanded-commercial-intelligence-strategy.md` — Phase 5 (ordering)

## Overview

Marketplace skill maturation (Alibaba/1688/Amazon candidate→stable), eBay baseline binding, PCS Scotland ingestion (truly new — IN-SCOPE per operator decision 2026-05-08), validator overlay binding, source-health canary cron. Each promoted source binds compatible worker_behavior_version + validator_behavior_version.

## Phase 5 Task Beads (9)

1. `l5-p5-validator-overlay-contract` — `skills/validation/overlay-customs-classification/` (decided early; binds all sources)
2. `l5-p5-alibaba-maturation` — Chinese retry + Ready-to-Ship + source-health
3. `l5-p5-1688-maturation` — Chinese-only short clusters + domestic wholesale floor
4. `l5-p5-amazon-maturation` — alias families + browser-first (post PA-API sunset)
5. `l5-p5-ebay-baseline-binding` — eBay benchmark-ceiling production binding
6. `l5-p5-pcs-scotland-skill` — full skill directory + daemon registry + bootstrap docs
7. `l5-p5-source-health-auth-binding` — source-health canary cron coverage
8. `l5-p5-marketplace-fixture-promotion-gate` — fixture mirror + candidate→stable journey
9. `l5-p5-skills-scope-preservation-gate` — phase review gate

## Gate Criteria (from strategy §Phase 5)

- Alibaba, 1688, Amazon, eBay baseline, and PCS satisfy the 6-file skill directory standard (`SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`) where production-bound.
- Alibaba/1688/Amazon move from candidate to stable only through canary, shadow/promotion, and validator gates.
- PCS canary proves Scotland ingestion scope without reopening the 2026-05-08 decision.
- Source-health canary cron covers marketplace and CDP dependency readiness.
- Quote-role semantics remain explicit and floor/ceiling rows do not silently become executable wholesale quotes.

## Roll Back If

- A marketplace source needs fabricated quotes to satisfy canaries.
- Candidate-stage output can become authoritative before stable promotion.
- PCS cannot be integrated while preserving Scotland coverage and the skill directory standard.

## Success Criteria

- All 5 production-bound marketplace + procurement sources (eBay, Alibaba, 1688, Amazon, PCS) have the full 6-file skill directory standard.
- Alibaba, 1688, Amazon at `promotion_stage: stable`; eBay stable (baseline); PCS at `stable` after canary pass.
- `skills/validation/overlay-customs-classification/` exists with 6-file standard; worker_behavior_version × validator_behavior_version compat matrix populated.
- `skills/source-health/l5-marketplace-commercial-intelligence/` exists; daily canary cron covers all 7 sources (Alibaba/1688/Amazon/PCS/SeaRates/Freightos/TARIC).
- `supplier_quotes` table contains rows from all 4 marketplaces with correct `price_signal_role`: `optimistic_floor` (Alibaba), `domestic_wholesale_floor` (1688), `benchmark_ceiling` (Amazon + eBay).
- `tests/fixtures/calibration/marketplace-search/{alibaba,1688,amazon,pcs}/{canary.inputs.jsonl, golden.assertions.yaml}` all present.
- `L5-T-freight_engine-009..014, 016, 018` + `L5-T-customs_classification-008, 011` (10 IDs) pass.
- PCS Scotland is unconditional (per operator decision 2026-05-08); not reopened.
- All 9 P5 task beads closed.

---

## L5 Phase 6: Gold-Set Accuracy, CBAM, And Integration

- id: `testproject1-hn05.6`
- type: epic
- priority: P0
- labels: implementation,l5,l5-phase-6,layer-5
- parent: `testproject1-hn05`

### Description

# L5 Phase 6: Gold-Set Accuracy, CBAM, And Integration

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — §1.2.1, §1.2.5, §1.14.1-1.14.3, §1.16.1, §1.16.2, §2.4.1, §3.1.11, §3.1.12, §3.2.1, §3.2.2, §3.3.18, §3.3.19, §3.4.1-3.4.3, §3.6.1-3.6.2, §3.7.3, §3.7.8, §3.8.1, §3.8.2, §3.10.1, §3.10.2 (PRIMARY)
- **TDD**: `...l5-expanded-commercial-intelligence-tdd.md` — §8.2 (`L5-T-customs_classification-009,010,012,025..032,035,036`), §8.3 (`L5-T-measure_interaction-020`), §8.5 (`L5-T-freight_engine-017`), §7 (CI-L5-001..008) (PRIMARY)
- **Strategy**: `...l5-expanded-commercial-intelligence-strategy.md` — Phase 6 (ordering)

## Overview

Phase 6 closes L5: gold-set accuracy harness for 3 categories (fasteners + packaging-cartons + examination-gloves), CBAM forward placeholder, 3 comparison adapters (Dutify/Zonos/Avalara — fixture-only), field-level projection boundary for automation-sensitive sentinels, decision-grade feature-flag `main.rs` cutover (shadow → delegate → default-on), Stage 11 live-loop integration, full DoD walkthrough. Largest single bead in L5 is the `main.rs` refactor (split across 3 beads: feature flag/shadow, delegation cutover, default-on/live demo) because the existing landed-cost path spans 208 textual touch points across 2650 lines.

## Phase 6 Task Beads (11)

1. `l5-p6-accuracy-harness-module` — `packages/intelligence/src/customs/accuracy.ts`
2. `l5-p6-comparison-adapters` — Dutify/Zonos/Avalara fixture-only adapters
3. `l5-p6-goldset-fasteners-corpus` — `tests/fixtures/calibration/customs-classification/fasteners/`
4. `l5-p6-goldset-packaging-cartons-corpus` — `tests/fixtures/calibration/customs-classification/packaging-cartons/`
5. `l5-p6-goldset-examination-gloves-corpus` — `tests/fixtures/calibration/customs-classification/examination-gloves/`
6. `l5-p6-cbam-placeholder` — CBAM exposure flag logic (2027-scope sectors)
7. `l5-p6-field-level-projection-boundary` — viewer-redacted view or RPC serializer
8. `l5-p6-mainrs-feature-flag-shadow` — default-OFF decision-grade flag + shadow comparison
9. `l5-p6-mainrs-delegation-cutover` — delegate Stage 11 landed-cost to package engines
10. `l5-p6-mainrs-default-on-and-live-demo` — default-ON + Contracts Finder live demo
11. `l5-p6-final-dod-readiness-gate` — full DoD #1-12 walkthrough + L5→L6 readiness fixture

## Gate Criteria (from strategy §Phase 6)

- All 5 L5 package test files + 3 daemon integration targets exist or are extended with L5 IDs.
- All 19 `packages/intelligence` production files exist in fixed 6/4/5/3/1 distribution.
- Gold-set corpora exist for 3 categories, preserve `clean/weird/mutated/pairwise` variants, pass evidence-tiered metrics.
- Comparison adapters are fixture/captured-sample only.
- CBAM exposure flag tests pass; quantified estimate fields remain null.
- Viewer/operator field-level projection passes `L5-T-measure_interaction-020`.
- Stage 11 live-loop integration runs old+new side-by-side under default-OFF flag, then delegates after regression gates pass.
- Mandatory Stage Validation Test Gate passes including every daemon integration target loop.
- DoD rows 1-12 verified in one walkthrough.

## Roll Back If

- A category can pass production using bronze-only labels.
- Decision-grade landed cost cannot preserve L4 sync/redaction semantics.
- Viewer role can read automation-sensitive sentinels.
- Full integration requires adding schema columns to `landed_cost_estimates`.
- L6 bid readiness can bypass active L5 blockers or floor-only quote evidence.

## Success Criteria

- Full Stage Validation Test Gate passes 100%.
- All 19 package files present in fixed distribution: 6 customs + 4 FX + 5 logistics + 3 validation + 1 accuracy (`accuracy.ts`).
- All 5 L5 package test files + 3 daemon integration targets exist.
- 3 gold-set corpora (fasteners, packaging-cartons, examination-gloves) preserve `clean/weird/mutated/pairwise` variants AND score `production_gate: eligible` per §11.6 accuracy harness.
- 3 fixture-only comparison adapters (Dutify/Zonos/Avalara); no live vendor account dependency.
- CBAM exposure flag tests pass; `cbam_estimate_gbp_pence` remains null until rate rules final.
- Field-level projection boundary: viewer cannot read automation-sensitive sentinel values by field name OR sentinel value; operator can; neither role mutates push-owned rows outside L4 RPC paths.
- `main.rs` Stage 11 landed-cost path delegated to package engines under `L5_ENABLE_DECISION_GRADE_LANDED_COST` flag (default-on after shadow comparison + Contracts Finder live demo + regression gates).
- All `CI-L5-001..008` + all 12 DoD rows verified in one walkthrough.
- L5→L6 readiness fixture proves every blocker condition.
- All 11 P6 task beads closed.

---

# Phase 1-6 Task Beads

## Phase 1 Task Beads

### l5-p1-sqlite-product-commodity-codes

- id: `testproject1-hn05.1.1`
- type: task
- priority: P0
- labels: l5,l5-phase-1,layer-5,migration
- parent: `testproject1-hn05.1`

#### Description

# l5-p1-sqlite-product-commodity-codes

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — §1.3.2, §2.4.2 (PRIMARY)
- **TDD**: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md` — `L5-T-customs_classification-006` (PRIMARY)
- **Strategy**: Phase 1, P1-A (foundation)
- **Bootstrap**: §10 (artifact checklist)

## Coordination Protocol

Use `bin/br-claim <bead-id>` and `bin/br-complete <bead-id>` per AGENTS.md Rule 2 (lines 21-33). Never call `br update --status` or `br close` directly. Sync via `bin/br-sync` only.

## Context

L5's single new local table: `product_commodity_codes` junction. Mirrors the Supabase projection table of the same name (created in L4) which becomes the push target. The local table records every recall→candidate mapping with source agreement metadata, rank, score, and stage-run identity. Reruns atomically set-replace prior rows per candidate. All 36 decision-grade columns on `landed_cost_estimates` already exist as nullable from L0 — NO DDL change to that table.

## Files to Create/Modify

- **Create:** `migrations/0013_l5_product_commodity_codes.sql` (or next-free `NNNN_l5_product_commodity_codes.sql` if `0013` slot was taken by a concurrent agent).

## Spec Requirements (verbatim)

**L5-FR-1.3.2 — L5 introduces the local SQLite `product_commodity_codes` junction.**
- AC1: L5 creates `product_commodity_codes` in `migrations/0013_l5_product_commodity_codes.sql` or the next-free `NNNN_l5_product_commodity_codes.sql` at implementation time.
- AC2: The Supabase projection table of the same name, created in L4, is the push target.
- AC3: Local rows use set-replacement semantics per candidate when classification reruns, matching the L5 classification lattice contract in `docs/strategy/layer-strategy.md` §10.

**L5-FR-2.4.2 — `product_commodity_codes` is actively populated from recall lattice output.**
- AC1: Each row links a `product_candidate` to a commodity code with recall source, agreement metadata, rank, score, and stage-run identity.
- AC2: Reruns atomically replace the candidate's prior commodity-code set.

## Implementation Details

Existing migration sequence in `migrations/`:
- `0001_sqlite_state_initial.sql` (L0 baseline; `landed_cost_estimates:322-394` lives here)
- `0003_sqlite_runtime.sql`
- `0004_l2_pending_operator_intents.sql`
- `0005_l3_operational_hardening.sql`
- `0006_pdf_extraction_toolkit.sql`
- `0007_net_margin_gate_assessments.sql`
- `0008_manifest_generation_columns.sql`
- `0009_operator_input_overrides.sql`
- `0010_operator_override_audit_rows.sql`
- `0011_commercial_basis_coverage.sql`
- `0012_l4_sync_state.sql`

Required columns (mirror the existing L4 Supabase projection table shape — read `infra/supabase/migrations/0001_projection_tables.sql` for the canonical projection-side definition):

| Column | Type | Notes |
|--------|------|-------|
| `id` | `text PRIMARY KEY` | UUID-style identifier |
| `product_candidate_id` | `text NOT NULL` | FK to `product_candidates` |
| `commodity_code` | `text NOT NULL` | HS6/CN8/UK10 code string |
| `recall_source_family` | `text NOT NULL CHECK (recall_source_family IN ('hmrc_fpo_or_search_suggestion','cpv_chapter_prior','rule_based_chapter_hint','hmrc_tariff_keyword_search','atr_codebook_reuse'))` | 5 families from §1.4.1 |
| `agreement_state` | `text NOT NULL CHECK (agreement_state IN ('agreed','diverged','unavailable'))` | per §11.2 trace contract |
| `rank` | `integer NOT NULL` | local shortlist rank |
| `retrieval_score_bps` | `integer NULL` | nullable when source family `unavailable` |
| `stage_run_id` | `text NOT NULL` | atomic commit cohort |
| `owner_id` | `text NOT NULL` | L4 push contract |
| `row_version` | `integer NOT NULL DEFAULT 1` | L4 push contract |
| `last_local_seq` | `integer NULL` | L4 push contract |
| `content_hash` | `text NOT NULL` | outbox replay hash |
| `created_at` | `text NOT NULL DEFAULT (datetime('now'))` |
| `updated_at` | `text NOT NULL DEFAULT (datetime('now'))` |

Required indices:
- `(product_candidate_id, stage_run_id)` — for set-replacement.
- `(commodity_code, recall_source_family)` — for cross-candidate aggregation in audits.

Set-replacement semantics: on classification rerun for `product_candidate_id = X` under `stage_run_id = Y`, the writer deletes all rows where `product_candidate_id = X` AND `stage_run_id < Y` (or older `row_version`) within the same SQLite transaction, then inserts the new rows. Test ID `L5-T-customs_classification-006` exercises this atomicity.

## Tests to Pass

- **`L5-T-customs_classification-006`** (T2, `L5StateDb` fixture): `product_commodity_codes` local table exists, mirrors the L4 Supabase projection shape, and set-replaces all rows for a candidate atomically under one `stage_run_id`.

Indirect coverage (must compile but assertions live elsewhere):
- `L5-T-customs_classification-036` — complete-row guard reads this table to populate `candidate_commodity_codes_json`.
- `CI-L5-001` (cargo check) and `CI-L5-005` (package inventory) require this migration to apply from scratch.

## Acceptance Criteria

- [ ] `migrations/0013_l5_product_commodity_codes.sql` exists and applies cleanly from a fresh DB.
- [ ] All required columns and indices present.
- [ ] Set-replacement is exercised by `L5-T-customs_classification-006`.
- [ ] L4 Supabase projection columns match local SQLite shape modulo SQLite/Postgres type differences (text vs varchar; integer vs bigint).
- [ ] Reapplying the migration on an already-migrated DB is a no-op.
- [ ] No new columns added to `landed_cost_estimates` (DDL diff guard).

## Dependencies

- **Blocked by:** none (parallel group P1 root).
- **Blocks:** `l5-p1-daemon-test-target-setup`, `l5-p4-tariff-graph-junction-writes`, `l5-p4-redaction-sentinel-design`, `l5-p1-scope-preservation-gate`.
- **Phase:** 1.

---

### l5-p1-error-vocabulary-enum

- id: `testproject1-hn05.1.2`
- type: task
- priority: P0
- labels: errors,l5,l5-phase-1,layer-5
- parent: `testproject1-hn05.1`

#### Description

# l5-p1-error-vocabulary-enum

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — §3.5.1-3.5.10 (PRIMARY)
- **TDD**: `...l5-expanded-commercial-intelligence-tdd.md` — §9.1 (ErrorCode enum coverage map), `L5-T-measure_interaction-018` (PRIMARY)
- **Strategy**: Phase 1; "Error Vocabulary Phase/Test Split" table

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`. Sync via `bin/br-sync`.

## Context

L5 adds 9 new `ErrorCode` enum variants (plus reuses existing `FpoAccessUnavailable`) to `crates/daemon/src/errors/manifest.rs`. The enum currently has 34 variants (lines 20-54). The matching `as_str()` and `parse_error_code` functions also update. Runtime emission paths are tested by domain phases (2, 3, 4); this bead only owns the vocabulary itself.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/errors/manifest.rs` (lines 20-55 enum + matching `as_str()` + `parse_error_code`).
- **Modify:** the same file's `impl ErrorCode { pub fn as_str(&self) -> &'static str }` matcher block and `parse_error_code` reverse mapping.

## Spec Requirements (verbatim)

Pre-amble from §3.5: "These customs error requirements are daemon error-vocabulary requirements, not skill-overlay-only codes. L5 updates `crates/daemon/src/errors/manifest.rs` (`ErrorCode` enum at line 20+ and `as_str()` matcher) plus any exhaustive match sites surfaced by `cargo clippy --all-targets -- -D warnings`, typically `validation/policy.rs`, `validation/executor.rs`, `errors/markdown.rs`, `tracing/structured.rs`, and `ipc/handlers/*`. The skill plug-and-play rule (L0 S4) governs skill manifests, not daemon enum vocabulary. `FPO_ACCESS_UNAVAILABLE` already exists as `FpoAccessUnavailable`; L5 reuses it and must not add a duplicate `FPO_UNAVAILABLE`."

**L5-FR-3.5.1 — `CLASSIFICATION_AMBIGUOUS`** — top HS6 candidates diverge across recall sources; action blocks auto-qualification and escalates to broker.
**L5-FR-3.5.2 — `TARIFF_BRANCH_DIVERGENCE`** — UK10 branches split on remedy/no-remedy or duty type; hard blocker, not scenario band.
**L5-FR-3.5.3 — `MEASURE_INTERACTION_UNKNOWN`** — unclear AD/CVD/safeguard interaction rules; manual customs review.
**L5-FR-3.5.4 — `EXPORTER_UNRESOLVED`** — active trade remedy with unidentified exporter/manufacturer; assumes residual rate and alerts operator.
**L5-FR-3.5.5 — `SUPPLEMENTARY_UNIT_MISSING`** — required supplementary unit data unavailable; abstain on that duty component.
**L5-FR-3.5.6 — `VALUATION_MODE_CONFLICT`** — material proxy/declaration divergence; flag uncertainty range.
**L5-FR-3.5.7 — `FPO_ACCESS_UNAVAILABLE`** — FPO/search-suggestion endpoint unreachable or access-restricted; continues with remaining sources + confidence penalty. AC2: reuse existing `FpoAccessUnavailable` enum variant; do not add duplicate `FPO_UNAVAILABLE`.
**L5-FR-3.5.8 — `TRADE_REMEDY_WATCH_STALE`** — TRA investigation or measure change affecting cached classification; requires reclassification.
**L5-FR-3.5.9 — `FX_RATE_MISSING`** — no valid HMRC customs FX snapshot; blocks landed-cost computation.
**L5-FR-3.5.10 — `FREIGHT_ESTIMATE_FAILED`** — neither Freightos nor SeaRates can quote; requires manual freight estimate, records warning/blocker.

## Implementation Details

Add to `pub enum ErrorCode` (already at lines 20-54), 9 new variants (preserve alphabetic/grouped insertion order consistent with existing style; the existing file groups by domain not strictly alphabetic):

```rust
// new L5 variants (insertion order can match the spec's §3.5 ordering)
ClassificationAmbiguous,
TariffBranchDivergence,
MeasureInteractionUnknown,
ExporterUnresolved,
SupplementaryUnitMissing,
ValuationModeConflict,
// FpoAccessUnavailable already exists at line ~32; do not add duplicate
TradeRemedyWatchStale,
FxRateMissing,
FreightEstimateFailed,
```

Matching `as_str()` arms (verbatim string constants — these are the wire-format identifiers used in error manifests, IPC, tracing, and the validator overlay rubric):

```rust
ErrorCode::ClassificationAmbiguous => "CLASSIFICATION_AMBIGUOUS",
ErrorCode::TariffBranchDivergence => "TARIFF_BRANCH_DIVERGENCE",
ErrorCode::MeasureInteractionUnknown => "MEASURE_INTERACTION_UNKNOWN",
ErrorCode::ExporterUnresolved => "EXPORTER_UNRESOLVED",
ErrorCode::SupplementaryUnitMissing => "SUPPLEMENTARY_UNIT_MISSING",
ErrorCode::ValuationModeConflict => "VALUATION_MODE_CONFLICT",
ErrorCode::TradeRemedyWatchStale => "TRADE_REMEDY_WATCH_STALE",
ErrorCode::FxRateMissing => "FX_RATE_MISSING",
ErrorCode::FreightEstimateFailed => "FREIGHT_ESTIMATE_FAILED",
```

`parse_error_code` (reverse string→variant) must accept the same 9 new strings. The existing `FPO_ACCESS_UNAVAILABLE` arm is left untouched.

## Tests to Pass

- **`L5-T-measure_interaction-018`** (T1, error inventory fixture): daemon error vocabulary contains `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `MEASURE_INTERACTION_UNKNOWN`, `EXPORTER_UNRESOLVED`, `SUPPLEMENTARY_UNIT_MISSING`, `VALUATION_MODE_CONFLICT`, existing `FPO_ACCESS_UNAVAILABLE`, `TRADE_REMEDY_WATCH_STALE`, `FX_RATE_MISSING`, `FREIGHT_ESTIMATE_FAILED`.

This bead also blocks (i.e., must exist before they can pass):
- `CI-L5-001` (cargo check), `CI-L5-002` (clippy with exhaustive matches), `CI-L5-007` (policy scan — no duplicate `FPO_UNAVAILABLE`).
- Phase 4 runtime tests that emit these codes: `L5-T-hmrc_fpo-005,008`, `L5-T-customs_classification-014,018,020,022-024`, `L5-T-measure_interaction-004,008,010,011,017`.
- Phase 2 `L5-T-fx_engine-007`, Phase 3 `L5-T-freight_engine-005`.

## Acceptance Criteria

- [ ] 9 new variants present in `ErrorCode` enum; 10th `FpoAccessUnavailable` left unchanged (no duplicate `FPO_UNAVAILABLE`).
- [ ] `as_str()` returns the verbatim screaming-snake-case strings listed above.
- [ ] `parse_error_code` accepts every new variant string.
- [ ] `cargo check --all-targets` passes for this enum-only change before exhaustive consumers update (i.e., enum addition is forward-compatible even before propagation, OR the propagation bead `l5-p1-error-exhaustive-consumers` MUST claim immediately after).
- [ ] `L5-T-measure_interaction-018` greppable variant set assertion passes.

## Dependencies

- **Blocked by:** none.
- **Blocks:** `l5-p1-error-exhaustive-consumers`, `l5-p1-scope-preservation-gate`, all Phase 2-4 runtime tests that emit these codes.
- **Phase:** 1.

---

### l5-p1-error-exhaustive-consumers

- id: `testproject1-hn05.1.3`
- type: task
- priority: P0
- labels: errors,l5,l5-phase-1,layer-5
- parent: `testproject1-hn05.1`
- blocked by: `testproject1-hn05.1.2`

#### Description

# l5-p1-error-exhaustive-consumers

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — §3.5 (preamble) (PRIMARY)
- **TDD**: `...l5-expanded-commercial-intelligence-tdd.md` — `L5-T-measure_interaction-018`, `CI-L5-002` (PRIMARY)
- **Strategy**: Phase 1; "Error Vocabulary Phase/Test Split"

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

After `l5-p1-error-vocabulary-enum` adds 9 new `ErrorCode` variants, `cargo clippy --all-targets -- -D warnings` will surface every exhaustive `match` on `ErrorCode` that is now non-exhaustive. This bead closes those sites typed-correctly (no generic string fallback, no `_ => ` wildcards that silently catch L5 variants). Spec §3.5 names the typical sites; the authoritative list is whatever clippy surfaces in this checkout.

## Files to Create/Modify

- **Modify (probable sites — confirm with clippy):**
  - `crates/daemon/src/validation/policy.rs`
  - `crates/daemon/src/validation/executor.rs`
  - `crates/daemon/src/errors/markdown.rs`
  - `crates/daemon/src/tracing/structured.rs`
  - `crates/daemon/src/ipc/handlers/*.rs`
  - Any test helper that pattern-matches on `ErrorCode`.

## Spec Requirements (verbatim)

From §3.5 preamble: "L5 updates `crates/daemon/src/errors/manifest.rs` (`ErrorCode` enum at line 20+ and `as_str()` matcher) plus any exhaustive match sites surfaced by `cargo clippy --all-targets -- -D warnings`, typically `validation/policy.rs`, `validation/executor.rs`, `errors/markdown.rs`, `tracing/structured.rs`, and `ipc/handlers/*`."

Each new variant must be classified at every match site by the semantic table in §3.5.1-3.5.10:

| Variant | Semantic Class | Runtime Action |
|---------|----------------|----------------|
| `ClassificationAmbiguous` | hard blocker | escalate to broker, no auto-qualification |
| `TariffBranchDivergence` | hard blocker | remedy/no-remedy or duty-type split |
| `MeasureInteractionUnknown` | hard blocker | manual customs review |
| `ExporterUnresolved` | typed soft blocker | apply residual rate, alert operator |
| `SupplementaryUnitMissing` | typed soft blocker | abstain on duty component |
| `ValuationModeConflict` | warning | uncertainty range |
| `TradeRemedyWatchStale` | hard blocker (freshness) | require reclassification |
| `FxRateMissing` | hard blocker | block landed-cost computation |
| `FreightEstimateFailed` | typed soft blocker | require manual freight estimate |
| `FpoAccessUnavailable` (existing) | recoverable | continue with remaining recall sources |

## Implementation Details

Workflow (mandatory):

1. Apply enum changes from `l5-p1-error-vocabulary-enum`.
2. `cargo clippy --all-targets -- -D warnings 2>&1 | rg "non-exhaustive patterns"` — gather full site list.
3. For each site, look up the surrounding semantic context (markdown rendering, IPC payload mapping, tracing field, validator policy decision) and add a typed arm using the table above.
4. Forbidden: `_ => Default::default()`, `_ => "unknown_error"`, or any wildcard that hides L5 codes from downstream classification.
5. Re-run `cargo clippy --all-targets -- -D warnings` to confirm zero warnings.

Common patterns:

- **markdown rendering (`errors/markdown.rs`):** each variant emits a human-readable bullet. Use the SCREAMING_SNAKE wire constant + a 1-sentence operator-visible explanation derived from the spec AC.
- **tracing (`tracing/structured.rs`):** typed field for `error_code` (already string) plus an additional structured `error_class` field for blocker vs warning.
- **validation policy (`validation/policy.rs`):** classify into `BLOCKER` vs `WARNING` vs `RECOVERABLE` per the table.
- **validation executor (`validation/executor.rs`):** propagate to verdict envelope; treat new hard blockers as `rejected` not `flagged` unless basis_sufficiency permits abstention.
- **IPC handlers (`ipc/handlers/*`):** ensure the new variant is serializable through the existing typed payload, not coerced to "other".

## Tests to Pass

- **`CI-L5-002`** (T1, repo workspace): `cargo clippy --all-targets -- -D warnings` exits `0` with exhaustive matches updated for all L5 error codes.
- **`L5-T-measure_interaction-018`** (T1, error inventory fixture): re-asserted from `l5-p1-error-vocabulary-enum`; this bead must keep it passing across all exhaustive sites.

## Acceptance Criteria

- [ ] `cargo clippy --all-targets -- -D warnings` returns 0.
- [ ] No wildcard `_ =>` arms over `ErrorCode` introduced by this bead.
- [ ] Each new variant has a semantically correct arm at every site clippy surfaced.
- [ ] `errors/markdown.rs` renders each new code with a 1-sentence operator-visible explanation that matches the spec AC.
- [ ] `tracing/structured.rs` emits structured `error_code` for each new variant.

## Dependencies

- **Blocked by:** `l5-p1-error-vocabulary-enum`.
- **Blocks:** `l5-p1-daemon-test-target-setup`, `l5-p1-scope-preservation-gate`, all Phase 2-4 runtime tests that emit these codes.
- **Phase:** 1.

---

### l5-p1-source-registry-shape

- id: `testproject1-hn05.1.4`
- type: task
- priority: P0
- labels: l5,l5-phase-1,layer-5,sources
- parent: `testproject1-hn05.1`

#### Description

# l5-p1-source-registry-shape

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — §1.15.2 (PCS), §2.3.4 (Amazon + Alibaba/1688 production bindings), §3.10.2 (PRIMARY)
- **TDD**: `...l5-expanded-commercial-intelligence-tdd.md` — `L5-T-freight_engine-012,014,016` (PRIMARY)
- **Strategy**: Phase 1
- **Bootstrap**: §10 (artifact checklist — Amazon and PCS source bindings)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Adds new `RegisteredSource` entries to `crates/daemon/src/sources/registry.rs` for L5 sources. Phase 1 ONLY adds the registry shape (struct entries) — actual skill files for marketplace skills are owned by Phase 5; PCS skill files are owned by Phase 5. This bead unlocks `RegisteredSource` referencing by Phase 5 work without forking dependency ordering.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/sources/registry.rs` (~947 lines, near the existing supplier and procurement-ingestion registrations).

## Spec Requirements (verbatim)

**L5-FR-2.3.4 — Marketplace source registration is explicit.**
- AC1: Alibaba and 1688 existing registry entries gain production bindings when promoted.
- AC2: Amazon gains a supplier source entry in `crates/daemon/src/sources/registry.rs` adjacent to Alibaba/1688 supplier registrations.
- AC3: Validator overlay binding records worker/validator behavior versions per source.

**L5-FR-1.15.2 — PCS Scotland ingestion is truly new and in scope.**
- AC2: `crates/daemon/src/sources/registry.rs` gains the PCS source entry near existing procurement ingestion sources after `procontract-enrich` and before supplier sources.

## Implementation Details

New registry entries to add (preserve existing field shape — read the existing `RegisteredSource` definition near lines 570-680 to learn the exact struct fields and whatever `category` / `interaction_profile` / `auth_requirements` typing is in use):

| New Entry | Adjacent To | Category | Notes |
|-----------|-------------|----------|-------|
| `pcs` | after `procontract-enrich-tender`, before supplier sources | procurement-ingestion | Scotland OCDS Web API |
| `amazon` (production binding) | after `1688` and before `ebay` | supplier-price-enrichment | benchmark-ceiling role; PA-API sunset 2026-04-30 acknowledged |
| `l5-marketplace-commercial-intelligence` | source-health canary slot | source-health | covers Alibaba/1688/Amazon/PCS/SeaRates/Freightos/TARIC |

Existing Alibaba and 1688 entries already exist (Phase 5 maturation adds production binding metadata + worker_behavior_version / validator_behavior_version fields).

Each entry must declare (whatever the existing shape mandates):
- `source_id` literal
- `skill_path` pointing to the skill directory
- `category` enum (supplier-price-enrichment, procurement-ingestion, source-health, landed-cost)
- `interaction_profile` (browser_cdp, http_public, etc.)
- `auth_requirements` (persistent_chrome_profile, none, etc.)
- `promotion_stage` (candidate/stable — Phase 5 promotes; Phase 1 leaves at candidate)
- Worker/validator behavior version fields per L4 contract — null until Phase 5 binds them.

## Tests to Pass

- **`L5-T-freight_engine-012`** (T2, source registry fixture): Alibaba/1688 production bindings are explicit, Amazon source entry is added near sibling supplier registrations, and worker/validator behavior versions are bound (field-present test passes in Phase 1; full binding closes in Phase 5).
- **`L5-T-freight_engine-014`** (T3, PCS canary): `skills/procurement-ingestion/pcs/` exists with full six-file standard and registry places it near procurement sources. (Registry-side assertion only — skill files closed by Phase 5.)
- **`L5-T-freight_engine-016`** (T2, cron fixture): marketplace source-health canary cron exercises Alibaba, 1688, Amazon, PCS, SeaRates, Freightos, and TARIC readiness — requires source registry entries for those sources to be addressable.

## Acceptance Criteria

- [ ] `pcs` registry entry placed after `procontract-enrich-tender` and before supplier sources.
- [ ] `amazon` production-binding entry placed adjacent to `alibaba` / `1688` / `ebay`.
- [ ] `l5-marketplace-commercial-intelligence` source-health entry exists.
- [ ] All entries compile against the existing `RegisteredSource` struct (no shape change).
- [ ] `cargo test -p daemon --bin daemon` passes for any registry-listing test.
- [ ] No `RegisteredSource` field is initialized to a fabricated worker/validator behavior version — Phase 5 owns binding.

## Dependencies

- **Blocked by:** none (parallel group P1 root).
- **Blocks:** `l5-p1-cron-definitions` (cron entries reference these source_id values), `l5-p1-scope-preservation-gate`, `l5-p5-pcs-scotland-skill`, `l5-p5-amazon-maturation`, `l5-p5-source-health-auth-binding`.
- **Phase:** 1.

---

### l5-p1-cron-definitions

- id: `testproject1-hn05.1.5`
- type: task
- priority: P0
- labels: cron,l5,l5-phase-1,layer-5
- parent: `testproject1-hn05.1`
- blocked by: `testproject1-hn05.1.4`

#### Description

# l5-p1-cron-definitions

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — §1.11.2 (PRIMARY)
- **TDD**: `...l5-expanded-commercial-intelligence-tdd.md` — `L5-T-fx_engine-009`, `L5-T-freight_engine-016`, `L5-T-customs_classification-023,024` (PRIMARY — runtime; this bead owns shape only)
- **Strategy**: Phase 1
- **Bootstrap**: §8 (cron configuration seeding — exact env vars + cron expressions)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Adds 4 `CronJobDefinition` entries to `crates/daemon/src/scheduler/cron.rs` following the existing `VALIDATION_CALIBRATION_READINESS_SWEEP_CRON` (`0 20 2 * * *`) and `VALIDATION_CROSS_MODEL_AUDIT_CRON` (`0 30 3 * * *`) pattern. Runtime behavior (work enqueued, stale enqueue, recalculation jobs) is exercised by Phase 2/4 tests; this bead owns the definition shape so the scheduler test surface can introspect them.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/scheduler/cron.rs` (current 363 lines; insertion points near existing `VALIDATION_*` constants and the cron job slice they populate).

## Spec Requirements (verbatim)

**L5-FR-1.11.2 — Freshness watchers enqueue recalculation or reclassification.**
- AC1: TRA investigation openings set `tradeRemedyWatchState = investigation_open` and enqueue affected commodity families for review.
- AC2: HMRC tariff structure or measure changes emit a tariff-change control event and enqueue affected cached classifications.
- AC3: FX monthly rollover enqueues affected landed-cost estimates for recalculation.
- AC4: Implementation adds four new `CronJobDefinition` entries in `crates/daemon/src/scheduler/cron.rs` following the existing `VALIDATION_CALIBRATION_READINESS_SWEEP_CRON` and `VALIDATION_CROSS_MODEL_AUDIT_CRON` pattern.

## Implementation Details

4 new cron entries (from bootstrap §8, verbatim):

| Const Name | Env Var | Cron Expression | Skill ID | Purpose |
|------------|---------|-----------------|----------|---------|
| `HMRC_TARIFF_NEWS_WEEKLY_CRON` | `L5_ENABLE_HMRC_TARIFF_NEWS_CRON` | `0 15 3 * * 1` | `landed-cost/hmrc-tariff-news-watch` | weekly HMRC tariff structure and measure update check |
| `TRA_CASE_REGISTER_DAILY_CRON` | `L5_ENABLE_TRA_CASE_REGISTER_CRON` | `0 45 2 * * *` | `landed-cost/tra-case-register-watch` | daily TRA investigation/provisional/definitive measure watcher |
| `HMRC_FX_MONTHLY_ROLLOVER_CRON` | `L5_ENABLE_HMRC_FX_ROLLOVER_CRON` | `0 10 4 18-25 * *` | `landed-cost/hmrc-fx-monthly-rollover` | check HMRC monthly FX publication window and enqueue affected recalculations |
| `MARKETPLACE_SOURCE_HEALTH_CANARY_CRON` | `L5_ENABLE_MARKETPLACE_SOURCE_HEALTH_CRON` | `0 0 5 * * *` | `source-health/l5-marketplace-commercial-intelligence` | daily source-health canaries for marketplace + CDP freight/TARIC skills |

Each entry follows the existing `CronJobDefinition` struct shape (see what `VALIDATION_CALIBRATION_READINESS_SWEEP_CRON` populates — `source_id`, `skill_id`, `skill_version`, `cron_expression`, `enabled_env`). All 4 entries share `source_id = "l5-commercial-intelligence"` per bootstrap §8 JSON seed.

The 4 cron payloads, once added to the registry slice, must:
- Be visible to whatever test surface enumerates cron definitions (`CronDefinitionHarness` in TDD §5).
- Honor the `enabled_env` gating — when the env var is unset/false, the cron is registered but inactive (matches existing pattern).
- Compile against the `RegisteredSource` entry `l5-marketplace-commercial-intelligence` added by `l5-p1-source-registry-shape`.

## Tests to Pass

- **`L5-T-fx_engine-009`** (T2, `CronDefinitionHarness`): `hmrc-fx-monthly-rollover` cron definition exists and dispatches affected `landed_cost_estimates` recalculation work (runtime test runs in Phase 2; definition shape exists from this bead).
- **`L5-T-freight_engine-016`** (T2, cron fixture): marketplace source-health canary cron exists and exercises Alibaba/1688/Amazon/PCS/SeaRates/Freightos/TARIC readiness without paid API auth checks (definition + skill_id + source_id assertions live here).
- **`L5-T-customs_classification-023`** (T2): TRA + HMRC tariff change cron entries exist; runtime emission lives in Phase 4 `classification-freshness.ts`.
- **`L5-T-customs_classification-024`** (T2): outage/missed-watcher cron behavior; same split.

## Acceptance Criteria

- [ ] 4 new constants present in `cron.rs` matching the exact strings above.
- [ ] All 4 cron entries appear in the registry slice that the scheduler enumerates.
- [ ] `enabled_env` gating works the same way as existing `VALIDATION_*` crons.
- [ ] `CronDefinitionHarness` can list all 4 by source_id `l5-commercial-intelligence` + the source-health binding.
- [ ] `cargo test -p daemon --bin daemon` passes for any cron-listing test.

## Dependencies

- **Blocked by:** `l5-p1-source-registry-shape`.
- **Blocks:** `l5-p1-daemon-test-target-setup`, `l5-p1-scope-preservation-gate`, `l5-p2-fx-cron-persistence-gate`, `l5-p4-classification-freshness`, `l5-p5-source-health-auth-binding`.
- **Phase:** 1.

---

### l5-p1-env-bootstrap-preflight

- id: `testproject1-hn05.1.6`
- type: task
- priority: P1
- labels: bootstrap,l5,l5-phase-1,layer-5
- parent: `testproject1-hn05.1`

#### Description

# l5-p1-env-bootstrap-preflight

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — §3.9.1, §3.9.2, §3.10.2 (PRIMARY)
- **TDD**: `...l5-expanded-commercial-intelligence-tdd.md` — `CI-L5-006` (PRIMARY)
- **Bootstrap**: §3 (env prereqs), §4 (external access contract), §5 (validation script), §10 (artifact checklist)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Wires the L5 bootstrap preflight: env-var defaults, public HTTP probes, CDP readiness check, paid-key scan (forbidden), prerequisite skill verification. Runs from repo root. Documentation env-var defaults live alongside the bootstrap doc. This bead does NOT add any paid commercial onboarding step.

## Files to Create/Modify

- **Modify (or create reference helper):** existing bootstrap doc remains the source of truth (`docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md`); this bead wires the actual env var defaults into whatever per-developer env file the project uses (`.envrc.example` or equivalent) and ensures the §5 preflight script lives at a stable repo path.
- **Confirm:** `bin/ensure-cdp-9222` exists and is callable.

## Spec Requirements (verbatim)

**L5-FR-3.9.1 — L5 depends on Layers 0-4 and skill-first external data access.**
- AC1: Layers 0-4 must be complete: pipeline data, validation infrastructure, sync engine, projection tables, and web portal.
- AC2: L5 external access is satisfied by existing skills, operator-built prerequisite skills, and public/free endpoints.

**L5-FR-3.9.2 — Operator-built prerequisites gate bead execution.**
- AC1: Phase 3 Freight beads cannot start until `skills/landed-cost/freightos-freight-estimate/` exists with `SKILL.md`, `result.schema.json`, and `checkpoint.schema.json`.
- AC2: Phase 4 Customs recall beads cannot start until `skills/landed-cost/taric-cross-check/` exists with `SKILL.md`, `result.schema.json`, and `checkpoint.schema.json`.
- AC3: These two prerequisite skills are not pipeline beads; step 09 audit verifies them before authorizing execution.
- AC4: Step 09 records missing `canary.inputs.jsonl`, `golden.assertions.yaml`, or `validator-rubric.yaml` as stable-production blockers even when the initial bead-claim prerequisite files exist.

**L5-FR-3.10.2 — L5 implements ADR-001 skill-based data retrieval for added sources.**
- AC1: Alibaba, 1688, Amazon, PCS, Freightos, SeaRates, TARIC, HMRC, and FX inputs use skills or public/free endpoints, not external commercial connector accounts.

## Implementation Details

Required env-var defaults (from bootstrap §3/§5):

| Var | Default | Purpose |
|-----|---------|---------|
| `HMRC_TRADE_TARIFF_BASE_URL` | `https://www.trade-tariff.service.gov.uk` | HMRC commodity/search probe |
| `HMRC_EXCHANGE_RATES_URL` | `https://www.trade-tariff.service.gov.uk/exchange_rates/monthly?year=<current>` | HMRC monthly FX |
| `FRANKFURTER_BASE_URL` | `https://api.frankfurter.app` | Frankfurter spot FX |
| `CDP_ENSURE_HELPER_PATH` | `bin/ensure-cdp-9222` | CDP readiness helper |

Forbidden env vars (the preflight rejects them per CI-L5-006):

`XE_API_KEY`, `FREIGHTOS_API_KEY`, `SEARATES_API_KEY`, `TARIC_SUPPORT_API_KEY`, `HS_PING_API_KEY`, `EASYSHIP_API_KEY`, `SHIPPO_API_KEY`, `CURRENCYBEACON_API_KEY`, `OANDA_API_KEY`, `DUTIFY_API_KEY`, `ZONOS_API_KEY`, `AVALARA_API_KEY`.

The new L5 cron-enable env vars (default to unset/false; cron registered but inactive):

`L5_ENABLE_HMRC_TARIFF_NEWS_CRON`, `L5_ENABLE_TRA_CASE_REGISTER_CRON`, `L5_ENABLE_HMRC_FX_ROLLOVER_CRON`, `L5_ENABLE_MARKETPLACE_SOURCE_HEALTH_CRON`.

Preflight script (committed to a stable repo path — `scripts/l5-preflight.sh` or extend the existing preflight infrastructure) implements bootstrap §5 verbatim:

- toolchain probes: cargo, bun, rg, curl, jq, ubs (warn if absent)
- file existence: spec, TDD, bootstrap, L4 RLS migration, `crates/daemon/src/db/sync.rs`
- `landed_cost_estimates` and `product_commodity_codes` already in L4 RLS + sync push set
- forbidden env scan
- HMRC v2 commodity probe (with `/uk/api` legacy fallback)
- HMRC search probe
- HMRC monthly FX page probe
- Frankfurter `latest?from=USD&to=GBP` probe
- CDP `/json/version` probe (via `bin/ensure-cdp-9222`)
- prerequisite skill file presence: Freightos + TARIC `SKILL.md` + `result.schema.json` + `checkpoint.schema.json`
- gold-set fixture seed dirs: `tests/fixtures/calibration/customs-classification/{fasteners,packaging-cartons,examination-gloves}/`

## Tests to Pass

- **`CI-L5-006`** (T1, policy scan): no paid API key env vars or signup/procurement steps appear in L5 implementation/bootstrap docs.
- Indirect: `CI-L5-001..005` depend on the preflight being green before they can be validated.

## Acceptance Criteria

- [ ] `.envrc.example` (or project's stable env-var file) lists the 4 default URLs above + 4 new L5 cron-enable vars defaulted unset/false.
- [ ] Preflight script lives at a stable repo path and exits non-zero on any forbidden env var.
- [ ] Preflight script verifies both Freightos and TARIC prerequisite skills exist before Phase 3/4 work claims.
- [ ] `rg "(XE|FREIGHTOS|SEARATES|TARIC_SUPPORT|HS_PING|EASYSHIP|SHIPPO|CURRENCYBEACON|OANDA|DUTIFY|ZONOS|AVALARA)_API_KEY" docs/pipeline/l5-expanded-commercial-intelligence/` returns empty.
- [ ] `bin/ensure-cdp-9222` is callable from preflight script.
- [ ] No vendor signup step appears in any L5 doc.

## Dependencies

- **Blocked by:** none.
- **Blocks:** `l5-p1-scope-preservation-gate`, all later phase beads that rely on env defaults.
- **Phase:** 1.

---

### l5-p1-daemon-test-target-setup

- id: `testproject1-hn05.1.7`
- type: task
- priority: P1
- labels: daemon-tests,l5,l5-phase-1,layer-5
- parent: `testproject1-hn05.1`
- blocked by: `testproject1-hn05.1.5`, `testproject1-hn05.1.3`, `testproject1-hn05.1.1`

#### Description

# l5-p1-daemon-test-target-setup

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — §3.2.2 (Stage Validation Test Gate) (PRIMARY)
- **TDD**: `...l5-expanded-commercial-intelligence-tdd.md` — §6 (Planned File Inventory), `CI-L5-005` (PRIMARY)
- **Strategy**: Phase 1 (test target homes)
- **Bootstrap**: §6.2 (test workspace layout)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Creates the 3 supplementary daemon Rust integration test files. Phase 1 owns entry-point setup (file exists, compiles, no `#[ignore]`); real assertions are populated by Phases 2/3/4/6 against the same files. This bead is the contract that those test targets WILL exist before downstream phases write to them.

## Files to Create/Modify

- **Create:** `crates/daemon/tests/customs_classification.rs`
- **Create:** `crates/daemon/tests/customs_fx_freight_engine.rs`
- **Create:** `crates/daemon/tests/customs_classification_accuracy.rs`

## Spec Requirements (verbatim)

From TDD §6 (Daemon Test Target table):

| Daemon Test Target | Must Exercise |
|--------------------|---------------|
| `crates/daemon/tests/customs_classification.rs` | tariff graph cache, `product_commodity_codes`, complete `landed_cost_estimates` population, daemon `ErrorCode` enum/matcher extension, measure interaction warnings/blockers, VAT/valuation conflict persistence, RLS/redaction sentinels, L5→L6 readiness blockers |
| `crates/daemon/tests/customs_fx_freight_engine.rs` | HMRC FX rollover cron, historical correction enqueue, `fx_basis_delta_bps` storage, Freightos/SeaRates/TARIC prerequisite checks, marketplace source-health cron, provider discrepancy warning persistence |
| `crates/daemon/tests/customs_classification_accuracy.rs` | gold/silver/bronze corpus scoring, abstention-positive-class metrics, mutation-family sweeps |

## Implementation Details

Each test file must:

1. Include the standard daemon test header (database setup helpers, fixture loaders from `tests/common/` or wherever existing files import them — see `crates/daemon/tests/sqlite_supabase_sync.rs` or similar L4 file for the pattern).
2. Declare at least one passing stub test that exercises the L4 base setup (so the file is not empty and doesn't fail to compile under `cargo test --test <name>`).
3. Be picked up by the mandatory Stage Validation Test Gate loop:
   ```bash
   for t in crates/daemon/tests/*.rs; do
     cargo test --test "$(basename "$t" .rs)" || exit 1
   done
   ```
4. Stub tests should NOT use `#[ignore]` or `#[should_panic]`. Bootstrap §12 forbids skipped tests. Stubs should be smoke-level: open `StateDb`, apply migrations including `0013_l5_product_commodity_codes.sql`, assert a count, return. Phase 2/3/4/6 beads then add real test functions to the same file.

The 3 test files MUST cite (in test function names or assertion labels) L5-T-* IDs they own. Phase 1 files just declare a smoke test; the bead inventory for which test ID lands where:

- `customs_classification.rs` → owns Rust-side coverage of: `L5-T-customs_classification-004,006,007,008,022,023,024,036`, `L5-T-measure_interaction-019,020`.
- `customs_fx_freight_engine.rs` → owns: `L5-T-fx_engine-005,006,009,010`, `L5-T-freight_engine-012,015,016`.
- `customs_classification_accuracy.rs` → owns: `L5-T-customs_classification-027..032,035`.

## Tests to Pass

- **`CI-L5-005`** (T1, repository inventory): exactly the 19 package additions under `packages/intelligence` exist, the 5 L5 package test files exist (proven separately by `l5-p2/3/4` beads), and the 3 supplementary daemon integration targets are present or explicitly extended with L5 IDs.

Smoke tests per file (one each, named to indicate L5 ownership without claiming domain content yet — e.g., `l5_customs_classification_smoke`, `l5_customs_fx_freight_engine_smoke`, `l5_customs_classification_accuracy_smoke`).

## Acceptance Criteria

- [ ] All 3 daemon integration test files exist and compile under `cargo check --all-targets`.
- [ ] Each file has at least one passing smoke test (no `#[ignore]`, no empty bodies, no existence-only assertions).
- [ ] The Stage Validation loop discovers all 3 files via `crates/daemon/tests/*.rs` and they all pass.
- [ ] Each smoke test applies migrations including `migrations/0013_l5_product_commodity_codes.sql` to confirm Phase 1 schema landed.
- [ ] File headers cite which L5-T-* IDs each file will own (as a comment block) so Phase 2-6 beads know where to add their real test functions.

## Dependencies

- **Blocked by:** `l5-p1-sqlite-product-commodity-codes`, `l5-p1-error-exhaustive-consumers`, `l5-p1-cron-definitions`.
- **Blocks:** `l5-p1-scope-preservation-gate`, all Phase 2/3/4/6 beads that target these files.
- **Phase:** 1.

---

### l5-p1-scope-preservation-gate

- id: `testproject1-hn05.1.8`
- type: task
- priority: P0
- labels: l5,l5-phase-1,layer-5,review-gate
- parent: `testproject1-hn05.1`
- blocked by: `testproject1-hn05.1.5`, `testproject1-hn05.1.4`, `testproject1-hn05.1.3`, `testproject1-hn05.1.2`, `testproject1-hn05.1.1`, `testproject1-hn05.1.7`, `testproject1-hn05.1.6`

#### Description

# l5-p1-scope-preservation-gate

## SOURCE OF TRUTH WARNING

- **Spec**: §§ covered by all Phase 1 beads (PRIMARY)
- **TDD**: §7 (CI gates), `L5-T-customs_classification-006`, `L5-T-measure_interaction-018`, `L5-T-fx_engine-009`, `L5-T-freight_engine-012,014,016` (PRIMARY)
- **Strategy**: Phase 1 gate criteria

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Phase 1 review gate. Closes the foundation phase by verifying every Phase 1 task bead produced its deliverable AND the integrated foundation passes static gates. This bead claims no new code; it's a verification pass. Phase 2-5 beads can begin claiming only after this gate is green.

## Files to Inspect (no modifications)

- All Phase 1 task bead outputs (see Phase 1 sub-epic for the list).

## Spec Requirements (gate criteria from strategy §Phase 1)

1. `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p daemon --bin daemon` all pass for Phase 1 scope.
2. `product_commodity_codes` local table applies from scratch; set-replacement is test-owned.
3. L5 error vocabulary contains 9 new daemon variants, reuses existing `FpoAccessUnavailable`, updates `parse_error_code`, closes all exhaustive propagation sites.
4. 4 L5 cron definitions visible through scheduler definition surface.
5. No paid API env var or procurement instruction appears in L5 bootstrap/strategy.

## Implementation Details (verification checklist — actions, not code)

This bead is verification-only. The implementing agent runs the following:

```bash
# 1. Foundation compiles green
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon

# 2. Migration applies cleanly to fresh DB
sqlite3 /tmp/l5-foundation-check.db < migrations/0001_sqlite_state_initial.sql
for m in migrations/000[2-9]*.sql migrations/001[0-3]*.sql; do
  sqlite3 /tmp/l5-foundation-check.db < "$m" || exit 1
done
sqlite3 /tmp/l5-foundation-check.db ".tables" | rg -q product_commodity_codes || exit 1

# 3. Error vocabulary contains all 9 new variants
rg -q "ClassificationAmbiguous" crates/daemon/src/errors/manifest.rs
rg -q "TariffBranchDivergence" crates/daemon/src/errors/manifest.rs
rg -q "MeasureInteractionUnknown" crates/daemon/src/errors/manifest.rs
rg -q "ExporterUnresolved" crates/daemon/src/errors/manifest.rs
rg -q "SupplementaryUnitMissing" crates/daemon/src/errors/manifest.rs
rg -q "ValuationModeConflict" crates/daemon/src/errors/manifest.rs
rg -q "TradeRemedyWatchStale" crates/daemon/src/errors/manifest.rs
rg -q "FxRateMissing" crates/daemon/src/errors/manifest.rs
rg -q "FreightEstimateFailed" crates/daemon/src/errors/manifest.rs
# Must NOT exist (duplicate guard from CI-L5-007)
! rg -q "FpoUnavailable\b" crates/daemon/src/errors/manifest.rs

# 4. 4 cron constants exist
for c in HMRC_TARIFF_NEWS_WEEKLY_CRON TRA_CASE_REGISTER_DAILY_CRON HMRC_FX_MONTHLY_ROLLOVER_CRON MARKETPLACE_SOURCE_HEALTH_CANARY_CRON; do
  rg -q "$c" crates/daemon/src/scheduler/cron.rs || exit 1
done

# 5. Source registry has new entries
rg -q '"pcs"' crates/daemon/src/sources/registry.rs
rg -q "l5-marketplace-commercial-intelligence" crates/daemon/src/sources/registry.rs

# 6. No paid API env var anywhere in L5 docs
! rg -q "(XE|FREIGHTOS|SEARATES|TARIC_SUPPORT|HS_PING|EASYSHIP|SHIPPO|CURRENCYBEACON|OANDA|DUTIFY|ZONOS|AVALARA)_API_KEY" docs/pipeline/l5-expanded-commercial-intelligence/
```

## Tests to Pass (this bead verifies, not implements)

- **`CI-L5-001`** (T1, cargo check)
- **`CI-L5-002`** (T1, cargo clippy)
- **`CI-L5-003`** (T1, cargo fmt)
- **`CI-L5-005`** (T1, repo inventory — daemon files exist)
- **`CI-L5-006`** (T1, policy scan)
- **`CI-L5-007`** (T1, no duplicate FPO_UNAVAILABLE)
- **`L5-T-customs_classification-006`** (T2, product_commodity_codes table + set-replacement)
- **`L5-T-measure_interaction-018`** (T1, error vocabulary inventory)
- **`L5-T-fx_engine-009`** (T2, hmrc-fx-monthly-rollover cron exists)
- **`L5-T-freight_engine-012`** (T2, source registry shape)
- **`L5-T-freight_engine-014`** (T3, PCS canary registry-side assertion)
- **`L5-T-freight_engine-016`** (T2, marketplace source-health cron exists)

## Acceptance Criteria

- [ ] All 6 verification commands above succeed (exit 0).
- [ ] All listed test IDs pass.
- [ ] No Phase 1 task bead is left in `in_progress` or open status.
- [ ] Roll-back conditions from strategy §Phase 1 are confirmed NOT triggered (no L4 sync semantics change; no generic string fallbacks; cron entries fit existing pattern).

## Dependencies

- **Blocked by:** `l5-p1-sqlite-product-commodity-codes`, `l5-p1-error-vocabulary-enum`, `l5-p1-error-exhaustive-consumers`, `l5-p1-source-registry-shape`, `l5-p1-cron-definitions`, `l5-p1-env-bootstrap-preflight`, `l5-p1-daemon-test-target-setup`.
- **Blocks:** Phase 2-5 first beads (each phase's first bead depends on this gate, except where the strategy explicitly allows parallelism).
- **Phase:** 1 (final).

---

## Phase 2 Task Beads

### l5-p2-fx-types-and-cassettes

- id: `testproject1-hn05.2.1`
- type: task
- priority: P0
- labels: fx,l5,l5-phase-2,layer-5
- parent: `testproject1-hn05.2`
- blocked by: `testproject1-hn05.1.8`

#### Description

# l5-p2-fx-types-and-cassettes

## SOURCE OF TRUTH WARNING

- **Spec**: §1.16.1, §2.1.1, §2.1.2 (PRIMARY)
- **TDD**: §5 (`L5TariffCassette`, `FxSnapshotFixture`), §8.4 (`L5-T-fx_engine-001,002,003`), §11.1 (HTTP cassette contract) (PRIMARY)
- **Strategy**: Phase 2 (FX types first)
- **Bootstrap**: §6.1 (package surface — 4 FX files), §6.2 (test workspace), §11.1

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Lays the FX engine foundation: shared TypeScript types under `packages/intelligence/src/fx/fx-types.ts` and the cassette directory shape under `tests/fixtures/cassettes/l5/fx/` + `tests/fixtures/l5/fx/`. No HTTP calls; no business logic. Phase 2 sequential ordering requires this bead to land before HMRC + Frankfurter parsers can claim, so all parsers consume identical type definitions.

## Files to Create/Modify

- **Create:** `packages/intelligence/src/fx/fx-types.ts` (FX shared types)
- **Create:** `packages/intelligence/src/fx/fx-engine.test.ts` (test file skeleton; full tests added by later P2 beads)
- **Create:** `tests/fixtures/cassettes/l5/fx/` directory tree with stub `hmrc-monthly-current.json` and `frankfurter-usd-gbp.json` cassettes following §11.1 contract
- **Create:** `tests/fixtures/l5/fx/` directory tree with the basis-delta fixture

## Spec Requirements (verbatim)

**L5-FR-1.16.1 — L5 adds customs, FX, logistics, validation, and accuracy modules under `packages/intelligence`.**
- AC2: Validation comparison adapters remain comparison-only and use fixtures or captured samples, with no external vendor credentials.

**L5-FR-2.1.1 — Dual FX uses HMRC customs rates and Frankfurter commercial spot rates.**
- AC1: `customsFxRate` comes from HMRC Exchange Rates for Customs and VAT monthly publication.
- AC2: `commercialFxRate` comes from Frankfurter through the existing `skills/landed-cost/simple-fx-lookup/` extension.
- AC3: No commercial FX account is required.

**L5-FR-2.1.2 — FX basis delta and rollover behavior are explicit.**
- AC1: `fxBasisDeltaBps` stores the customs/commercial rate gap in basis points.
- AC2: In-flight estimates use the HMRC monthly rate effective for the calculation timestamp; if rollover publishes before decision, the estimate is marked stale and recalculated.
- AC3: HMRC corrections for a historical month create a new rate snapshot, preserve the prior snapshot for audit, and enqueue affected historical estimates.

## Implementation Details

`fx-types.ts` exports:

```ts
export interface FxSnapshot {
  basis: "hmrc_customs_monthly" | "frankfurter_commercial_spot";
  effectiveMonth?: string; // YYYY-MM (HMRC monthly only)
  effectiveDate?: string;  // YYYY-MM-DD (Frankfurter spot only)
  baseCurrency: "USD" | "EUR" | "GBP" | "CNY";
  quoteCurrency: "GBP";
  rate: number;             // unit base-currency per quote-currency
  rateBps: number;          // rate in basis points = rate * 10000
  sourceUrl: string;
  checkedAt: string;        // ISO 8601 timestamp
  snapshotId: string;       // stable identifier for audit linkage
}

export interface FxBasisDelta {
  customsSnapshot: FxSnapshot;
  commercialSnapshot: FxSnapshot;
  basisDeltaBps: number;    // (commercialRate - customsRate) / customsRate * 10000
  computedAt: string;
}

export type FxRecalculationTrigger =
  | { kind: "monthly_rollover"; previousMonth: string; newMonth: string }
  | { kind: "historical_correction"; affectedMonth: string; previousSnapshotId: string; newSnapshotId: string }
  | { kind: "basis_delta_threshold_crossed"; thresholdBps: number; observedBps: number };
```

Cassette shape (per TDD §11.1 — exact JSON schema):

```json
{
  "case_id": "<unique-id>",
  "endpoint_family": "<hmrc_customs_monthly|frankfurter_commercial_spot>",
  "request": {
    "method": "GET",
    "url": "<full url>",
    "headers": { "accept": "application/json", "user-agent": "testproject1-l5-cassette-recorder/1" },
    "body_sha256": null,
    "authenticated": false
  },
  "request_fingerprint": "<canonical fingerprint>",
  "response": {
    "status": 200,
    "headers": { "content-type": "..." },
    "body_path": "<relative path>",
    "body_sha256": "sha256:<hex>"
  },
  "recorded_at": "<ISO8601>",
  "recorder": "scripts/manual curl -LfsS, no auth, no paid key",
  "expected_failure_mode": null
}
```

Required initial cassettes:

- `tests/fixtures/cassettes/l5/fx/hmrc-monthly-current.json` (success snapshot)
- `tests/fixtures/cassettes/l5/fx/frankfurter-usd-gbp.json` (success snapshot, `.rates.GBP` present)
- Per TDD §11.1, failure cassettes are also required (HMRC missing month, malformed CSV/XML, Frankfurter 429/5xx, missing `.rates.GBP`) — these can land here or in the parser beads, but the cassette tree structure must be ready.

Test file skeleton (`packages/intelligence/src/fx/fx-engine.test.ts`):

```ts
import { describe, it, expect } from "vitest";
import type { FxSnapshot, FxBasisDelta, FxRecalculationTrigger } from "./fx-types.js";

// [TEST_START] - bootstrap §11 logging requirement
// L5-T-fx_engine-* tests land here, populated by:
// - l5-p2-hmrc-customs-fx-parser  (001, 005, 006, 007, 009)
// - l5-p2-frankfurter-commercial-fx (002, 008, 011)
// - l5-p2-fx-basis-recalculation  (003, 004, 010, 012)

describe("fx-engine types compile", () => {
  it("FxSnapshot, FxBasisDelta, FxRecalculationTrigger structural smoke", () => {
    const snapshot: FxSnapshot = {
      basis: "hmrc_customs_monthly",
      effectiveMonth: "2026-05",
      baseCurrency: "USD",
      quoteCurrency: "GBP",
      rate: 0.7834,
      rateBps: 7834,
      sourceUrl: "https://www.trade-tariff.service.gov.uk/exchange_rates/monthly?year=2026",
      checkedAt: "2026-05-19T00:00:00Z",
      snapshotId: "hmrc-2026-05-USD-GBP-v1",
    };
    expect(snapshot.rateBps).toBe(7834);
  });
});
```

The skeleton must NOT mark tests `.skip` or `#[ignore]` — bootstrap §12 forbids that.

## Tests to Pass

- Indirect coverage at this stage; this bead lays scaffolding. The skeleton smoke must pass via `bun test packages/intelligence/src/fx/fx-engine.test.ts`.
- Cassette schema must validate against TDD §11.1 — file presence + JSON parse + required fields present.

## Acceptance Criteria

- [ ] `packages/intelligence/src/fx/fx-types.ts` exists and exports `FxSnapshot`, `FxBasisDelta`, `FxRecalculationTrigger`.
- [ ] `packages/intelligence/src/fx/fx-engine.test.ts` exists with one passing smoke test, lists all 12 L5-T-fx_engine-* IDs by comment header showing which P2 bead populates them.
- [ ] Cassette directory tree present under `tests/fixtures/cassettes/l5/fx/` + `tests/fixtures/l5/fx/`.
- [ ] Initial success cassettes match §11.1 schema (request, request_fingerprint, response with body_path + body_sha256).
- [ ] `bun run build` passes; `bun test packages/intelligence/src/fx/fx-engine.test.ts` passes the smoke.
- [ ] `packages/intelligence/scripts/lint.js` passes (no app/daemon/Supabase/desktop refs).

## Dependencies

- **Blocked by:** `l5-p1-scope-preservation-gate` (Phase 1 foundation green).
- **Blocks:** `l5-p2-hmrc-customs-fx-parser`, `l5-p2-frankfurter-commercial-fx`, `l5-p2-fx-cron-persistence-gate`.
- **Phase:** 2.

---

### l5-p2-hmrc-customs-fx-parser

- id: `testproject1-hn05.2.2`
- type: task
- priority: P0
- labels: fx,l5,l5-phase-2,layer-5
- parent: `testproject1-hn05.2`
- blocked by: `testproject1-hn05.2.1`

#### Description

# l5-p2-hmrc-customs-fx-parser

## SOURCE OF TRUTH WARNING

- **Spec**: §2.1.1, §2.1.2, §2.1.3, §3.1.8, §3.5.9 (PRIMARY)
- **TDD**: §8.4 `L5-T-fx_engine-001,005,006,007` (PRIMARY)
- **Strategy**: Phase 2 (HMRC parser lane parallel to Frankfurter after types land)
- **Bootstrap**: §5 (preflight HMRC monthly probe), §10 (artifact checklist)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

HMRC monthly customs FX is the LEGALLY AUTHORITATIVE rate for customs duty calculation. This parser fetches the HMRC monthly publication, parses snapshot fields, detects monthly rollover, and detects historical corrections. Public/free endpoint — no API key. Emits `FX_RATE_MISSING` (existing daemon enum from Phase 1) when no valid snapshot exists.

## Files to Create/Modify

- **Create:** `packages/intelligence/src/fx/hmrc-exchange-rates.ts`
- **Modify:** `packages/intelligence/src/fx/fx-engine.test.ts` (add real assertions for `L5-T-fx_engine-001,005,006,007`)
- **Add fixtures:** failure cassettes in `tests/fixtures/cassettes/l5/fx/` per §11.1 (missing-month, malformed CSV/XML)

## Spec Requirements (verbatim)

**L5-FR-2.1.1 — Dual FX uses HMRC customs rates and Frankfurter commercial spot rates.**
- AC1: `customsFxRate` comes from HMRC Exchange Rates for Customs and VAT monthly publication.

**L5-FR-2.1.2 — FX basis delta and rollover behavior are explicit.**
- AC2: In-flight estimates use the HMRC monthly rate effective for the calculation timestamp; if rollover publishes before decision, the estimate is marked stale and recalculated.
- AC3: HMRC corrections for a historical month create a new rate snapshot, preserve the prior snapshot for audit, and enqueue affected historical estimates.

**L5-FR-2.1.3 — FX error paths block unsafe landed-cost computation.**
- AC1: HMRC missing month or parse failure emits `FX_RATE_MISSING` when no valid customs rate snapshot exists.

**L5-FR-3.1.8 — DoD 8: HMRC Exchange Rate Service provides customs valuation FX.**
- AC1: Monthly customs FX snapshots populate landed-cost estimates.

**L5-FR-3.5.9 — Error code `FX_RATE_MISSING`.**
- AC1: Trigger is no valid HMRC customs FX snapshot; action blocks landed-cost computation.

## Implementation Details

`hmrc-exchange-rates.ts` exports:

```ts
import type { FxSnapshot } from "./fx-types.js";

export interface HmrcMonthlyFetcher {
  // Fetch HMRC monthly rate for (baseCurrency -> GBP) effective in given month
  fetchSnapshot(opts: {
    baseCurrency: "USD" | "EUR" | "CNY";
    effectiveMonth: string; // YYYY-MM
    cassettePath?: string;  // for deterministic unit tests
  }): Promise<FxSnapshot>;

  // Detect monthly rollover: new month published after previous snapshot was taken
  detectRollover(previous: FxSnapshot, now: Date): { rolledOver: boolean; newMonth?: string };

  // Detect historical correction: same effectiveMonth, different rate
  detectCorrection(previous: FxSnapshot, fresh: FxSnapshot): { corrected: boolean };
}
```

Behavior:

- Parse HMRC monthly publication. The publication can be HTML/CSV/XML — verify the live format with `curl "$HMRC_EXCHANGE_RATES_URL"`; if a structured `/api/exchange_rates` endpoint is available, prefer it; otherwise parse HTML/CSV deterministically.
- Snapshot ID format: `hmrc-<YYYY-MM>-<BASE>-GBP-v<n>` where `n` increments on historical correction.
- Missing month or parse failure emits `FX_RATE_MISSING` via the daemon error vocabulary (the package surface returns a tagged result; daemon-side translation already exists from `l5-p1-error-exhaustive-consumers`).
- Live HTTP smoke tests under `T3` tier; deterministic unit tests use cassettes from `l5-p2-fx-types-and-cassettes`.

Tests to add to `fx-engine.test.ts` (per TDD §8.4):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-fx_engine-001` | T1 | HMRC monthly CSV cassette parses with effective month, source URL, checked timestamp, no API key |
| `L5-T-fx_engine-005` | T2 | rollover fixture: new HMRC month before decision → estimates marked stale, recalculation jobs enqueued with old/new snapshot IDs |
| `L5-T-fx_engine-006` | T2 | correction fixture: historical HMRC correction creates new snapshot, preserves prior, enqueues historical estimates |
| `L5-T-fx_engine-007` | T1 | missing-HMRC fixture: missing month or parse failure → `FX_RATE_MISSING` emitted; no fabricated rate |

Each test name MUST contain its primary ID verbatim (TDD §2: "Every runnable package test name must include its primary stable `L5-T-*` ID verbatim").

## Tests to Pass

- **`L5-T-fx_engine-001`** (T1)
- **`L5-T-fx_engine-005`** (T2)
- **`L5-T-fx_engine-006`** (T2)
- **`L5-T-fx_engine-007`** (T1)

## Acceptance Criteria

- [ ] `hmrc-exchange-rates.ts` exists, exports `HmrcMonthlyFetcher`-style API.
- [ ] All 4 listed test IDs pass.
- [ ] Snapshot IDs stable across reads of the same cassette.
- [ ] No live HMRC HTTP call in T1/T2 tier tests — all unit tests cassette-driven.
- [ ] T3 live smoke uses `HMRC_EXCHANGE_RATES_URL` from preflight env vars; transient 429/5xx warns + cassette fallback.
- [ ] `FX_RATE_MISSING` error code (string match) emitted on missing/parse-failure path; verifiable from test fixtures.

## Dependencies

- **Blocked by:** `l5-p2-fx-types-and-cassettes`.
- **Blocks:** `l5-p2-fx-basis-recalculation`, `l5-p2-simple-fx-skill-extension`, `l5-p2-fx-cron-persistence-gate`.
- **Phase:** 2.

---

### l5-p2-frankfurter-commercial-fx

- id: `testproject1-hn05.2.3`
- type: task
- priority: P0
- labels: fx,l5,l5-phase-2,layer-5
- parent: `testproject1-hn05.2`
- blocked by: `testproject1-hn05.2.1`

#### Description

# l5-p2-frankfurter-commercial-fx

## SOURCE OF TRUTH WARNING

- **Spec**: §2.1.1, §2.1.3, §3.1.9 (PRIMARY)
- **TDD**: §8.4 `L5-T-fx_engine-002,008,011` (PRIMARY)
- **Strategy**: Phase 2 (parallel to HMRC parser after types land)
- **Bootstrap**: §5 (Frankfurter preflight), §3 (`FRANKFURTER_BASE_URL`)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Frankfurter (ECB-backed public free FX) provides commercial spot rates for margin estimation — NOT for customs duty calculation (HMRC monthly remains legal authority). The package file name is `frankfurter-commercial-rates.ts` (note: the spec sometimes uses the legacy name `xe-commercial-rates.ts` from the original research doc, but operator decision recorded 2026-05-08 chose Frankfurter as the commercial FX source; the strategy §19-File Package Surface Routing names the implementation file `frankfurter-commercial-rates.ts`).

## Files to Create/Modify

- **Create:** `packages/intelligence/src/fx/frankfurter-commercial-rates.ts`
- **Modify:** `packages/intelligence/src/fx/fx-engine.test.ts` (add `L5-T-fx_engine-002,008,011`)
- **Add fixtures:** Frankfurter cassettes (429/5xx, malformed JSON, missing `.rates.GBP`) per TDD §11.1

## Spec Requirements (verbatim)

**L5-FR-2.1.1 — Dual FX uses HMRC customs rates and Frankfurter commercial spot rates.**
- AC2: `commercialFxRate` comes from Frankfurter through the existing `skills/landed-cost/simple-fx-lookup/` extension.
- AC3: No commercial FX account is required.

**L5-FR-2.1.3 — FX error paths block unsafe landed-cost computation.**
- AC2: Frankfurter rate-limit or downtime uses the last valid commercial snapshot only if TTL-valid; otherwise commercial margin confidence is reduced and blocker/warning state is recorded.

**L5-FR-3.1.9 — DoD 9: Frankfurter commercial FX provides spot rates.**
- AC1: Commercial spot FX populates `commercial_fx_rate` and `fx_basis_delta_bps`.

## Implementation Details

`frankfurter-commercial-rates.ts` exports:

```ts
import type { FxSnapshot } from "./fx-types.js";

export interface FrankfurterFetcher {
  fetchSpot(opts: {
    baseCurrency: "USD" | "EUR" | "CNY";
    cassettePath?: string;
  }): Promise<FxSnapshot>;

  // TTL-based last-valid retention
  getLastValid(opts: { baseCurrency: string; maxAgeSeconds: number }): FxSnapshot | null;
}
```

Endpoint contract:

- `GET $FRANKFURTER_BASE_URL/latest?from=USD&to=GBP` with redirects enabled
- Parse `.rates.GBP` — body MUST contain that field; if absent, fail with the documented error class (missing-field, NOT a generic fallback).
- TTL behavior: cache last successful snapshot in-memory or local fixture-friendly store; on downtime/429/5xx, return last-valid only if `now - checkedAt < maxAgeSeconds`.
- Confidence: if last-valid is returned (downtime path), the package surface marks `commercial_estimate_confidence_bps` reduction in the FxBasisDelta consumer (Phase 2.4 wires the reduction; this parser just exposes the staleness flag).

Tests to add (TDD §8.4):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-fx_engine-002` | T1 | Frankfurter JSON cassette: parses `.rates.GBP`, populates `commercial_fx_rate` with source/date evidence |
| `L5-T-fx_engine-008` | T1 | Frankfurter downtime fixture: last-valid used within TTL; otherwise commercial margin confidence drops, warning/blocker recorded |
| `L5-T-fx_engine-011` | T3 | live public probes: HMRC + Frankfurter reachable without credentials; transient outage warns + cassette fallback (HMRC half covered here; Frankfurter half lives here) |

## Tests to Pass

- **`L5-T-fx_engine-002`** (T1)
- **`L5-T-fx_engine-008`** (T1)
- **`L5-T-fx_engine-011`** (T3)

## Acceptance Criteria

- [ ] `frankfurter-commercial-rates.ts` exists with the `FrankfurterFetcher`-style API.
- [ ] Test function names include their primary `L5-T-fx_engine-*` ID verbatim.
- [ ] Body MUST parse via `.rates.GBP`; missing field is a failure mode, not a fallback.
- [ ] TTL behavior tested: stale snapshot returned only when `maxAgeSeconds` not exceeded.
- [ ] `T3` smoke uses `FRANKFURTER_BASE_URL` env var; transient 429/5xx → `[WARN]` + cassette fallback per TDD §5.1.
- [ ] No paid FX provider env var referenced.

## Dependencies

- **Blocked by:** `l5-p2-fx-types-and-cassettes`.
- **Blocks:** `l5-p2-fx-basis-recalculation`, `l5-p2-simple-fx-skill-extension`, `l5-p2-fx-cron-persistence-gate`.
- **Phase:** 2.

---

### l5-p2-fx-basis-recalculation

- id: `testproject1-hn05.2.4`
- type: task
- priority: P0
- labels: fx,l5,l5-phase-2,layer-5
- parent: `testproject1-hn05.2`
- blocked by: `testproject1-hn05.2.3`, `testproject1-hn05.2.2`

#### Description

# l5-p2-fx-basis-recalculation

## SOURCE OF TRUTH WARNING

- **Spec**: §2.1.2, §2.1.4, §2.4.1, §3.1.11, §3.3.16, §3.10.1, §3.10.2 (PRIMARY)
- **TDD**: §8.4 `L5-T-fx_engine-003,004,010,012`, §7.3 (Golden Arithmetic Constants — `FX_DELTA_RECALC_THRESHOLD_BPS = 150`) (PRIMARY)
- **Strategy**: Phase 2 (basis delta after HMRC + Frankfurter parsers land)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Owns `packages/intelligence/src/fx/fx-engine.ts` — selects the correct rate per use case (customs vs commercial), computes `fx_basis_delta_bps`, triggers margin recalculation when `abs(basis_delta_bps)` crosses the threshold constant. Persists snapshots into `landed_cost_estimates` via the L4 push contract — no schema change.

## Files to Create/Modify

- **Create:** `packages/intelligence/src/fx/fx-engine.ts`
- **Modify:** `packages/intelligence/src/fx/fx-engine.test.ts` (add `L5-T-fx_engine-003,004,010,012`)

## Spec Requirements (verbatim)

**L5-FR-2.1.2 — FX basis delta and rollover behavior are explicit.**
- AC1: `fxBasisDeltaBps` stores the customs/commercial rate gap in basis points.

**L5-FR-2.1.4 — FX basis delta can trigger margin recalculation or review.**
- AC1: When `abs(fx_basis_delta_bps)` crosses the Step 03 TDD threshold, the engine recomputes `net_margin_low_bps`, `net_margin_base_bps`, and `net_margin_high_bps` under both customs and commercial FX bases.
- AC2: The selected threshold is a named test constant in `fx-engine.test.ts`; untested hardcoded FX-delta thresholds are prohibited.

**L5-FR-2.4.1 — L5 populates every `landed_cost_estimates` column or preserves its L4 sync semantics.**
- AC1: Every row maps each L0 DDL column to the requirement that populates or preserves it.
- AC2: No listed column is treated as dead schema during L5 implementation.

**L5-FR-3.1.11 — DoD 11: decision-grade landed cost with full audit trail.**
- AC1: Each validated category has traceable customs, FX, freight, VAT, confidence, warning, and blocker evidence.

**L5-FR-3.3.16 — Scenario: Frankfurter commercial spot and FX basis delta.**
- AC1: Frankfurter spot rate populates `commercial_fx_rate`, computes `fx_basis_delta_bps`, and triggers the configured FX-delta margin recalculation when threshold is crossed.

**L5-FR-3.10.1 — L5 implements ADR-006 internal landed-cost engine at decision-grade depth.**
**L5-FR-3.10.2 — L5 implements ADR-001 skill-based data retrieval for added sources.**

## Implementation Details

`fx-engine.ts` exports:

```ts
import type { FxSnapshot, FxBasisDelta, FxRecalculationTrigger } from "./fx-types.js";
import type { HmrcMonthlyFetcher } from "./hmrc-exchange-rates.js";
import type { FrankfurterFetcher } from "./frankfurter-commercial-rates.js";

export interface FxEngine {
  // Select rate per use case
  rateForCustoms(baseCurrency: string, effectiveMonth: string): Promise<FxSnapshot>;
  rateForCommercial(baseCurrency: string): Promise<FxSnapshot>;

  // Compute basis delta
  computeBasisDelta(customs: FxSnapshot, commercial: FxSnapshot): FxBasisDelta;

  // Recalculation trigger: emit when threshold crossed or rollover/correction detected
  evaluateRecalculation(
    delta: FxBasisDelta,
    thresholdBps: number
  ): FxRecalculationTrigger | null;

  // Compute net margin band under both bases for stress check
  netMarginBandUnderBothBases(
    netMarginBase: { low: number; base: number; high: number },
    customsRate: number,
    commercialRate: number
  ): { underCustoms: { low: number; base: number; high: number }; underCommercial: { low: number; base: number; high: number } };
}
```

Golden constants (TDD §7.3):

```ts
export const FX_DELTA_RECALC_THRESHOLD_BPS = 150;
```

Basis-delta math (TDD §8.4 fixture for `L5-T-fx_engine-003`):

- customs rate `0.7834`, commercial rate `0.8070` → `fx_basis_delta_bps = round((0.8070 - 0.7834) / 0.7834 * 10000) = 301`
- Document the rounding rule explicitly: `bankers' rounding` (round-half-to-even) OR `round-half-up`. Bootstrap §11.3 names "named rounding rule"; pick one and pin it in the test as a constant.

Tests to add (TDD §8.4):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-fx_engine-003` | T1 | basis delta fixture: customs 0.7834 + commercial 0.8070 → `fx_basis_delta_bps = 301` via named rounding rule |
| `L5-T-fx_engine-004` | T1 | threshold fixture: `FX_DELTA_RECALC_THRESHOLD_BPS = 150` is a named test constant; crossing recomputes low/base/high net margins under customs and commercial bases |
| `L5-T-fx_engine-010` | T2 | DB write fixture: `customs_fx_rate`, `commercial_fx_rate`, `fx_basis_delta_bps`, `fx_rate_checked_at` persist to `landed_cost_estimates` and sync normally |
| `L5-T-fx_engine-012` | T1 | ADR fixture: dual public/free FX satisfies ADR-006 without external commercial FX procurement |

## Tests to Pass

- **`L5-T-fx_engine-003`** (T1)
- **`L5-T-fx_engine-004`** (T1)
- **`L5-T-fx_engine-010`** (T2)
- **`L5-T-fx_engine-012`** (T1)

## Acceptance Criteria

- [ ] `fx-engine.ts` exists and exports `FxEngine`-style API.
- [ ] `FX_DELTA_RECALC_THRESHOLD_BPS = 150` exported as a named constant; no hardcoded literal in arithmetic paths.
- [ ] Basis-delta arithmetic deterministic per documented rounding rule.
- [ ] All 4 listed test IDs pass.
- [ ] `landed_cost_estimates` columns `customs_fx_rate`, `commercial_fx_rate`, `fx_basis_delta_bps`, `fx_rate_checked_at` persist via the existing L4 sync path (no new column added; no separate sync channel).

## Dependencies

- **Blocked by:** `l5-p2-hmrc-customs-fx-parser`, `l5-p2-frankfurter-commercial-fx`.
- **Blocks:** `l5-p2-fx-cron-persistence-gate`, `l5-p4-vat-additional-code-valuation`, `l5-p4-conservative-defaults-evidence-leverage`, `l5-p4-divergence-readiness-producers`.
- **Phase:** 2.

---

### l5-p2-simple-fx-skill-extension

- id: `testproject1-hn05.2.5`
- type: task
- priority: P1
- labels: fx,l5,l5-phase-2,layer-5,skills
- parent: `testproject1-hn05.2`
- blocked by: `testproject1-hn05.2.3`, `testproject1-hn05.2.2`

#### Description

# l5-p2-simple-fx-skill-extension

## SOURCE OF TRUTH WARNING

- **Spec**: §2.1.1 AC2 (PRIMARY)
- **TDD**: `L5-T-fx_engine-002,011` (PRIMARY)
- **Strategy**: Phase 2 (skill extension lands after parsers + types fixed)
- **Bootstrap**: §13 Notes For Implementation: "The existing `skills/landed-cost/simple-fx-lookup/` uses an older Frankfurter endpoint in its current text. L5 implementation should update it to the public latest-rate endpoint or preserve a documented compatibility fallback while tests target `latest?from=USD&to=GBP`."

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Extends the existing skill `skills/landed-cost/simple-fx-lookup/` (currently 1799 bytes SKILL.md + 641 bytes result.schema.json + no checkpoint.schema.json) to:

1. Add HMRC monthly customs FX source path alongside the existing Frankfurter commercial path.
2. Update Frankfurter endpoint to `latest?from=<base>&to=GBP` and align with `frankfurter-commercial-rates.ts` parser.
3. Add `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml` to bring it to the L4 production skill directory standard (per §1.15 + §2.3.5 §1 last line).

## Files to Create/Modify

- **Modify:** `skills/landed-cost/simple-fx-lookup/SKILL.md` (extend to dual-rate)
- **Modify:** `skills/landed-cost/simple-fx-lookup/result.schema.json` (add HMRC monthly path output shape)
- **Create:** `skills/landed-cost/simple-fx-lookup/checkpoint.schema.json`
- **Create:** `skills/landed-cost/simple-fx-lookup/canary.inputs.jsonl`
- **Create:** `skills/landed-cost/simple-fx-lookup/golden.assertions.yaml`
- **Create:** `skills/landed-cost/simple-fx-lookup/validator-rubric.yaml`

## Spec Requirements (verbatim)

**L5-FR-2.1.1 — Dual FX uses HMRC customs rates and Frankfurter commercial spot rates.**
- AC2: `commercialFxRate` comes from Frankfurter through the existing `skills/landed-cost/simple-fx-lookup/` extension.
- AC3: No commercial FX account is required.

## Implementation Details

The skill SKILL.md frontmatter must preserve existing keys (skill_id, version, model_profile, etc.) and add:

```yaml
interaction_profile: http_public
result_schema_version: 2
auth_requirements: none
sources:
  - id: frankfurter
    url_template: "https://api.frankfurter.app/latest?from={base}&to=GBP"
    role: commercial_spot
  - id: hmrc_monthly
    url_template: "https://www.trade-tariff.service.gov.uk/exchange_rates/monthly?year={year}"
    role: customs_monthly
```

Result schema additions:

```json
{
  "type": "object",
  "required": ["snapshots", "checked_at"],
  "properties": {
    "snapshots": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["basis", "base_currency", "quote_currency", "rate", "rate_bps", "source_url"],
        "properties": {
          "basis": { "enum": ["hmrc_customs_monthly", "frankfurter_commercial_spot"] },
          "effective_month": { "type": "string", "pattern": "^\\d{4}-\\d{2}$" },
          "effective_date": { "type": "string", "format": "date" },
          "base_currency": { "type": "string" },
          "quote_currency": { "const": "GBP" },
          "rate": { "type": "number" },
          "rate_bps": { "type": "integer" },
          "source_url": { "type": "string" },
          "snapshot_id": { "type": "string" }
        }
      }
    },
    "checked_at": { "type": "string", "format": "date-time" }
  }
}
```

Canary inputs (`canary.inputs.jsonl`) — at least 3 cases per bootstrap §7.2 canary structure pattern (file uses JSONL with at least 5 records for marketplace; for landed-cost simple-fx-lookup, 3 minimum is acceptable since the existing skill is operational):

```jsonl
{"case_id":"l5-fx-usd-current","base_currency":"USD","mode":"both"}
{"case_id":"l5-fx-eur-current","base_currency":"EUR","mode":"both"}
{"case_id":"l5-fx-cny-current","base_currency":"CNY","mode":"both"}
```

Golden assertions (`golden.assertions.yaml`):

```yaml
l5-fx-usd-current:
  result_schema_valid: true
  has_snapshots_count: 2  # commercial + customs
  has_basis_hmrc_customs_monthly: true
  has_basis_frankfurter_commercial_spot: true
  rate_bps_positive: true
  source_urls_present: true
l5-fx-eur-current: { ... same shape ... }
l5-fx-cny-current: { ... same shape ... }
```

Validator rubric (`validator-rubric.yaml`) — overlay binding for FX correctness audits:

```yaml
overlay_id: validation/overlay-fx-lookup
worker_skill_id: landed-cost/simple-fx-lookup
checks:
  - field: rate_bps_positive
    severity: blocker
  - field: source_url_matches_basis
    severity: blocker
  - field: snapshot_id_unique
    severity: warning
```

Backward compatibility: existing callers that consume the older single-rate result shape can still consume the array's `frankfurter_commercial_spot` element. If absolute backward compatibility is required, write a brief 1-line migration note in SKILL.md (no code shim).

## Tests to Pass

- **`L5-T-fx_engine-002`** (T1) — covers Frankfurter path via skill or direct parser
- **`L5-T-fx_engine-011`** (T3) — live public probes (skill paths inherit the env-var probe behavior)

## Acceptance Criteria

- [ ] All 4 skill-directory-standard files exist (checkpoint.schema.json, canary.inputs.jsonl, golden.assertions.yaml, validator-rubric.yaml).
- [ ] SKILL.md frontmatter declares both source roles.
- [ ] Result schema validates with at least one fixture per basis.
- [ ] Frankfurter URL pattern uses `latest?from={base}&to=GBP`.
- [ ] No paid endpoint reference in any of the 6 skill files.
- [ ] `bun run test` includes any test that imports the skill metadata.

## Dependencies

- **Blocked by:** `l5-p2-hmrc-customs-fx-parser`, `l5-p2-frankfurter-commercial-fx`.
- **Blocks:** `l5-p2-fx-cron-persistence-gate`.
- **Phase:** 2.

---

### l5-p2-fx-cron-persistence-gate

- id: `testproject1-hn05.2.6`
- type: task
- priority: P0
- labels: l5,l5-phase-2,layer-5,review-gate
- parent: `testproject1-hn05.2`
- blocked by: `testproject1-hn05.2.4`, `testproject1-hn05.2.3`, `testproject1-hn05.2.2`, `testproject1-hn05.2.1`, `testproject1-hn05.1.5`, `testproject1-hn05.2.5`

#### Description

# l5-p2-fx-cron-persistence-gate

## SOURCE OF TRUTH WARNING

- **Spec**: §1.11.2 AC3, §2.1, §3.5.9, §3.1.8, §3.1.9 (PRIMARY)
- **TDD**: `L5-T-fx_engine-001..012`, `CI-L5-004` (PRIMARY)
- **Strategy**: Phase 2 gate

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Phase 2 review gate. Verifies the dual FX engine is integrated, the HMRC monthly rollover cron dispatches work via the daemon test surface, and `landed_cost_estimates` persistence is exercised end-to-end through L4 sync (no new column, no separate channel).

## Files to Inspect/Modify

- All Phase 2 task bead outputs.
- **Possibly extend:** `crates/daemon/tests/customs_fx_freight_engine.rs` (added in Phase 1 as a stub) with FX cron + persistence integration tests citing `L5-T-fx_engine-005,006,009,010`.

## Gate Criteria (from strategy §Phase 2)

1. `bun run build` and `bun run test` include `fx-engine.test.ts`.
2. `L5-T-fx_engine-001..012` pass.
3. HMRC monthly rollover cron dispatches recalculation work in the daemon test surface.
4. A fixture row persists `customs_fx_rate`, `commercial_fx_rate`, `fx_basis_delta_bps`, `fx_rate_checked_at`.
5. `FX_RATE_MISSING` behavior is typed and does not fabricate a customs FX snapshot.

## Implementation Details

This bead wires the daemon-side integration test (in `customs_fx_freight_engine.rs`) and the cron-dispatch assertion. Daemon-side tests required:

```rust
// in crates/daemon/tests/customs_fx_freight_engine.rs (extending Phase 1 stub)

#[test]
fn l5_t_fx_engine_009_hmrc_fx_monthly_rollover_cron_dispatches() {
    // Setup: open StateDb with L5 migrations applied
    // Seed a landed_cost_estimates row pinned to last month's customs FX snapshot
    // Tick scheduler past the HMRC_FX_MONTHLY_ROLLOVER_CRON expression
    // Assert: a recalculation work item was enqueued for the affected row
}

#[test]
fn l5_t_fx_engine_010_landed_cost_persists_dual_fx_fields() {
    // Setup: open StateDb, seed product_candidate + supplier_quote
    // Call into the package-layer fx-engine via the daemon's existing integration shim
    // Assert: customs_fx_rate, commercial_fx_rate, fx_basis_delta_bps, fx_rate_checked_at all populated
    // Assert: L4 outbox row created with the correct entity_kind = "landed_cost_estimates"
}
```

Verification checklist (this bead's actions):

```bash
bun run build
bun test packages/intelligence/src/fx/fx-engine.test.ts | rg -q "L5-T-fx_engine-001.*PASS"
# 12 ID greppable assertions:
for i in 001 002 003 004 005 006 007 008 009 010 011 012; do
  bun test packages/intelligence/src/fx/fx-engine.test.ts | rg -q "L5-T-fx_engine-$i" || exit 1
done

# CI-L5-004 — Bun build/test exits 0 and includes the 5 L5 Vitest files (this phase contributes fx-engine.test.ts)
bun run build && bun run test

# Daemon integration target
cargo test --test customs_fx_freight_engine
```

## Tests to Pass

- **`L5-T-fx_engine-001..012`** (12 IDs, T1/T2/T3)
- **`CI-L5-004`** (T1, Bun workspace exits 0 and includes the 5 L5 Vitest files — fx-engine.test.ts is one of them)

## Acceptance Criteria

- [ ] All 12 `L5-T-fx_engine-*` IDs pass.
- [ ] `customs_fx_freight_engine.rs` contains at least 2 new test functions covering FX cron + persistence.
- [ ] `cargo test --test customs_fx_freight_engine` passes.
- [ ] No new columns added to `landed_cost_estimates`.
- [ ] `FX_RATE_MISSING` is emitted only when no valid snapshot exists; never fabricated.
- [ ] `bun run build` passes.
- [ ] Phase 2 roll-back conditions NOT triggered: HMRC + Frankfurter both parse deterministically from public/free data or cassettes; no paid commercial FX dependency reintroduced; no hidden mutable state.

## Dependencies

- **Blocked by:** `l5-p2-fx-types-and-cassettes`, `l5-p2-hmrc-customs-fx-parser`, `l5-p2-frankfurter-commercial-fx`, `l5-p2-fx-basis-recalculation`, `l5-p2-simple-fx-skill-extension`, `l5-p1-cron-definitions`.
- **Blocks:** Phase 4 FX-gated lanes (`l5-p4-vat-additional-code-valuation`, `l5-p4-conservative-defaults-evidence-leverage`, `l5-p4-divergence-readiness-producers`), `l5-p6-final-dod-readiness-gate`.
- **Phase:** 2 (final).

---

## Phase 3 Task Beads

### l5-p3-freightos-prerequisite-check

- id: `testproject1-hn05.3.1`
- type: task
- priority: P0
- labels: freight,l5,l5-phase-3,layer-5,prerequisite
- parent: `testproject1-hn05.3`
- blocked by: `testproject1-hn05.1.8`

#### Description

# l5-p3-freightos-prerequisite-check

## SOURCE OF TRUTH WARNING

- **Spec**: §3.9.2 AC1, AC3, AC4 (PRIMARY)
- **TDD**: `L5-T-freight_engine-015` (PRIMARY)
- **Strategy**: Phase 3 prerequisite

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Operator-built skill `skills/landed-cost/freightos-freight-estimate/` is a HARD prerequisite gate for Phase 3 beads. Verified to exist at step 07 pre-condition pass; this bead re-runs the verification before any Phase 3 implementation work proceeds. Records missing canary/golden/rubric files as stable-production blockers per §3.9.2 AC4.

## Files to Inspect (no modifications)

- `skills/landed-cost/freightos-freight-estimate/SKILL.md`
- `skills/landed-cost/freightos-freight-estimate/result.schema.json`
- `skills/landed-cost/freightos-freight-estimate/checkpoint.schema.json` (may be missing — see below)
- `skills/landed-cost/freightos-freight-estimate/canary.inputs.jsonl` (stable-production prereq; recorded as blocker if missing)
- `skills/landed-cost/freightos-freight-estimate/golden.assertions.yaml` (stable-production prereq)
- `skills/landed-cost/freightos-freight-estimate/validator-rubric.yaml` (stable-production prereq)

## Spec Requirements (verbatim)

**L5-FR-3.9.2 — Operator-built prerequisites gate bead execution.**
- AC1: Phase 3 Freight beads cannot start until `skills/landed-cost/freightos-freight-estimate/` exists with `SKILL.md`, `result.schema.json`, and `checkpoint.schema.json`.
- AC3: These two prerequisite skills are not pipeline beads; step 09 audit verifies them before authorizing execution.
- AC4: Step 09 records missing `canary.inputs.jsonl`, `golden.assertions.yaml`, or `validator-rubric.yaml` as stable-production blockers even when the initial bead-claim prerequisite files exist.

## Implementation Details

Verification script (this bead executes, does not modify):

```bash
freightos_dir="skills/landed-cost/freightos-freight-estimate"
for f in SKILL.md result.schema.json checkpoint.schema.json; do
  test -f "$freightos_dir/$f" || { echo "[FAIL] missing $f"; exit 1; }
done

# Stable-production prereqs — record blocker but do NOT halt Phase 3 implementation work
for f in canary.inputs.jsonl golden.assertions.yaml validator-rubric.yaml; do
  if ! test -f "$freightos_dir/$f"; then
    echo "[STABLE-PRODUCTION BLOCKER] $f missing — must close before Phase 5 promotion or Phase 6 gold-set proof"
  fi
done

# Schema sanity
jq -e '.required and .properties' "$freightos_dir/result.schema.json" >/dev/null || exit 1
if test -f "$freightos_dir/checkpoint.schema.json"; then
  jq -e '.' "$freightos_dir/checkpoint.schema.json" >/dev/null || exit 1
fi

# Schema fields the freight engine adapter consumes
jq -e '.properties | (has("freight_low_gbp_pence") and has("freight_base_gbp_pence") and has("freight_high_gbp_pence")) or (has("low") and has("base") and has("high"))' "$freightos_dir/result.schema.json" >/dev/null || \
  echo "[WARN] Freightos result schema does not yet expose freight low/base/high pence fields — Phase 3 adapter must normalize"
```

If at the time of execution `checkpoint.schema.json` is missing — the step 07 verification revealed both operator-built skills currently lack this file. The bead body's expected behavior:

1. If `SKILL.md` + `result.schema.json` present but `checkpoint.schema.json` missing: STOP and escalate via AgentMail to operator with subject `[L5 BLOCKER] Operator-built skill prerequisite missing: skills/landed-cost/freightos-freight-estimate/checkpoint.schema.json`. Phase 3 implementation beads cannot CLAIM until the file is provided.
2. If all 3 hard prerequisites present and 1+ stable-production prereqs missing: continue Phase 3 implementation but record the gap in this bead's completion notes and ensure Phase 5/6 gates surface it.

## Tests to Pass

- **`L5-T-freight_engine-015`** (T2, external dependency fixture): Freightos and TARIC operator-built prerequisites exist before Phase 3/4 claims and missing canary/golden/rubric files remain stable-production blockers.

## Acceptance Criteria

- [ ] `SKILL.md`, `result.schema.json`, `checkpoint.schema.json` all exist for Freightos (otherwise this bead is BLOCKED, not complete).
- [ ] Missing stable-production prereqs (canary/golden/rubric) recorded as blockers in the bead body without halting Phase 3 implementation work.
- [ ] `result.schema.json` is JSON-Schema-valid and exposes (or is documented to need adapter normalization for) freight low/base/high pence fields.
- [ ] AgentMail escalation sent if any hard prerequisite is missing.

## Dependencies

- **Blocked by:** `l5-p1-scope-preservation-gate` (Phase 1 foundation green).
- **Blocks:** `l5-p3-logistics-types-mode-selector`, `l5-p3-freightos-adapter`.
- **Phase:** 3.

---

### l5-p3-logistics-types-mode-selector

- id: `testproject1-hn05.3.2`
- type: task
- priority: P0
- labels: freight,l5,l5-phase-3,layer-5
- parent: `testproject1-hn05.3`
- blocked by: `testproject1-hn05.3.1`

#### Description

# l5-p3-logistics-types-mode-selector

## SOURCE OF TRUTH WARNING

- **Spec**: §2.2.1, §3.6.2 (PRIMARY)
- **TDD**: §8.5 `L5-T-freight_engine-001,002` (PRIMARY)
- **Strategy**: Phase 3 (types + mode selector before adapters)
- **Bootstrap**: §6.1 (5 logistics files)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Freight engine foundation: shared TypeScript types + deterministic mode selector. Hard locks override cheaper modes when safety/feasibility requires (hazmat, battery, reefer, oversized, fragile). The 5 logistics files mandated by §1.16:

| File | Owner |
|------|-------|
| `freight-types.ts` | this bead |
| `mode-selector.ts` | this bead |
| `freightos-skill-adapter.ts` | `l5-p3-freightos-adapter` |
| `searates-skill-adapter.ts` | `l5-p3-searates-adapter` |
| `freight-engine.ts` | `l5-p3-freight-boundary-defaults` |

## Files to Create/Modify

- **Create:** `packages/intelligence/src/logistics/freight-types.ts`
- **Create:** `packages/intelligence/src/logistics/mode-selector.ts`
- **Create:** `packages/intelligence/src/logistics/freight-engine.test.ts` (skeleton; tests added by other P3 beads)

## Spec Requirements (verbatim)

**L5-FR-2.2.1 — Freight mode selection covers sea, air, express, and multimodal with hard locks.**
- AC1: Mode selection considers weight, volume, value, urgency, product type, hazmat, oversized, fragile, reefer, batteries, and packaging state.
- AC2: Hard locks override cheaper modes when safety or feasibility requires.

**L5-FR-3.6.2 — Open questions have present-tense runtime behavior.**
- AC1: Each open question includes current L5 behavior that a live implementation follows while the research-backed question remains open.
- AC2: Runtime behavior cannot wait for an open question to close; it must choose a conservative default, blocker, or explicit warning path.

## Implementation Details

`freight-types.ts` exports:

```ts
export type ShippingMode = "sea_lcl" | "sea_fcl" | "air_general" | "air_express" | "multimodal";

export interface ModeSelectionFacts {
  weightKg: number;
  volumeM3: number;
  valueGbpPence: number;
  urgency: "standard" | "expedited" | "critical";
  productType: string;            // head noun + family
  hazmat: boolean;
  oversized: boolean;
  fragile: boolean;
  reefer: boolean;                // temperature-controlled
  batteries: "none" | "loose" | "in_device" | "lithium_unknown";
  packagingState: "palletized" | "loose_cartons" | "unknown";
}

export interface ModeSelection {
  mode: ShippingMode;
  confidenceBps: number;          // 0-10000
  hardLocksTriggered: HardLock[];
  reason: string;                  // human-readable selection reason
  fallbackBoundaryUsed: boolean;   // true when missing facts forced boundary scenario
}

export type HardLock =
  | { kind: "hazmat"; details: string }
  | { kind: "battery"; details: string }
  | { kind: "reefer"; details: string }
  | { kind: "oversized"; details: string }
  | { kind: "fragile"; details: string };

export interface FreightEstimate {
  provider: "freightos" | "searates" | "manual";
  mode: ShippingMode;
  lowGbpPence: number | null;
  baseGbpPence: number | null;
  highGbpPence: number | null;
  evidenceTranscriptPath?: string;
  failureCode?: string;             // provider-specific (e.g., "FREIGHTOS_NO_QUOTE", "SEARATES_SCHEMA_DRIFT")
  checkedAt: string;
}

export interface PackingScenario {
  packagingState: "palletized" | "loose_cartons" | "unknown";
  scenariosToRun: ("palletized" | "loose_cartons")[];
}
```

`mode-selector.ts` exports `selectMode(facts: ModeSelectionFacts): ModeSelection` with these decision rules:

| Condition | Hard Lock | Resulting Mode |
|-----------|-----------|----------------|
| `hazmat = true` and `airOk = false` | hazmat-air-prohibited | `sea_lcl` or `sea_fcl` (size-dependent) |
| `batteries = "loose"` or `batteries = "lithium_unknown"` | battery-air-restricted | `sea_*` |
| `reefer = true` | reefer-required | `sea_fcl` (reefer container) or `air_general` (low volume reefer) |
| `oversized = true` | oversized-prohibited-on-express | exclude `air_express` |
| `fragile = true` and `urgency = critical` | fragile-rapid-handling | `air_general` |

Size-based default:

- `volumeM3 < 0.5` AND `urgency = critical` → `air_express`
- `volumeM3 < 5` AND `urgency = expedited` → `air_general`
- `volumeM3 < 15` → `sea_lcl`
- `volumeM3 >= 15` → `sea_fcl`

When `packagingState = "unknown"` (open question §3.6.2): bootstrap §4.2 mandates "highest-cost feasible freight boundary when category-specific heuristics are not calibrated; block automated completion if weight/volume evidence is insufficient." The mode selector emits `fallbackBoundaryUsed = true` and selects the higher-cost of {palletized, loose_cartons} mode that satisfies hard locks.

Tests to add (TDD §8.5):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-freight_engine-001` | T1 | sea, air, express, multimodal selection considers weight, volume, value, urgency, product type, hazmat, oversized, fragile, reefer, batteries, packaging state |
| `L5-T-freight_engine-002` | T1 | hazmat, battery, reefer, oversized, or fragile hard locks override cheaper modes when feasibility requires |

## Tests to Pass

- **`L5-T-freight_engine-001`** (T1)
- **`L5-T-freight_engine-002`** (T1)

## Acceptance Criteria

- [ ] `freight-types.ts` and `mode-selector.ts` exist and export the typed surface above.
- [ ] All 11 mode-selection facts from §2.2.1 AC1 are accepted as inputs.
- [ ] Hard locks override cheaper modes per the rule table.
- [ ] Open-question runtime behavior: unknown packaging → highest-cost boundary, `fallbackBoundaryUsed = true`.
- [ ] Test function names contain primary L5-T-* IDs verbatim.
- [ ] `bun run build` passes; package lint boundary preserved.

## Dependencies

- **Blocked by:** `l5-p3-freightos-prerequisite-check`.
- **Blocks:** `l5-p3-freightos-adapter`, `l5-p3-searates-adapter`, `l5-p3-freight-boundary-defaults`, `l5-p3-freight-source-health-gate`.
- **Phase:** 3.

---

### l5-p3-freightos-adapter

- id: `testproject1-hn05.3.3`
- type: task
- priority: P0
- labels: adapters,freight,l5,l5-phase-3,layer-5
- parent: `testproject1-hn05.3`
- blocked by: `testproject1-hn05.3.2`, `testproject1-hn05.3.1`

#### Description

# l5-p3-freightos-adapter

## SOURCE OF TRUTH WARNING

- **Spec**: §2.2.2, §2.2.4, §3.1.10 (PRIMARY)
- **TDD**: §8.5 `L5-T-freight_engine-003,008` (PRIMARY)
- **Strategy**: Phase 3 (parallel with SeaRates adapter after types land)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Wraps the operator-built `skills/landed-cost/freightos-freight-estimate/` CDP browser skill and normalizes its output into the standard `FreightEstimate` shape. Records source-specific failure codes when Freightos returns no quote, page shape drifts, or schema breaks.

## Files to Create/Modify

- **Create:** `packages/intelligence/src/logistics/freightos-skill-adapter.ts`
- **Modify:** `packages/intelligence/src/logistics/freight-engine.test.ts` (add `L5-T-freight_engine-003` partial coverage + `L5-T-freight_engine-008` Freightos half)

## Spec Requirements (verbatim)

**L5-FR-2.2.2 — Freight estimates use Freightos and SeaRates CDP skills.**
- AC1: The engine invokes `skills/landed-cost/freightos-freight-estimate/` and `skills/landed-cost/searates-freight-estimate/` for dual-source cross-check.
- AC3: Freight access remains skill-first/CDP-browser based and requires no commercial connector account.

**L5-FR-2.2.4 — Freight error paths are source-specific.**
- AC1: Freightos no quote, unsupported mode, or page/schema drift is recorded with source name and transcript evidence.

**L5-FR-3.1.10 — DoD 10: Freightos + SeaRates CDP skills provide freight market data.**
- AC1: Both provider adapters normalize low/base/high freight estimates or record source-specific failure.

## Implementation Details

`freightos-skill-adapter.ts` exports:

```ts
import type { FreightEstimate, ModeSelection } from "./freight-types.js";

export interface FreightosAdapter {
  // Invoke the CDP skill; returns FreightEstimate or typed failure
  estimate(opts: {
    mode: ModeSelection["mode"];
    originPort: string;
    destinationPort: string;
    weightKg: number;
    volumeM3: number;
    cassettePath?: string;       // deterministic unit tests
  }): Promise<FreightEstimate>;
}
```

Behavior:

- Spawn the existing `skills/landed-cost/freightos-freight-estimate/` skill via the standard skill-invocation path (whatever the existing `searates-freight-estimate` invocation does is the template — read its existing skill runner integration in the daemon).
- Parse the skill's `result.schema.json`-conformant output; if the field shape uses `low_amount_gbp_pence` / `base_amount_gbp_pence` / `high_amount_gbp_pence` (or similar), normalize to `lowGbpPence`/`baseGbpPence`/`highGbpPence` per `FreightEstimate` shape.
- Source-specific failure codes emitted:
  - `FREIGHTOS_NO_QUOTE` — skill returned successfully but no quote band
  - `FREIGHTOS_UNSUPPORTED_MODE` — selected mode not offered
  - `FREIGHTOS_PAGE_SHAPE_DRIFT` — DOM selectors mismatched
  - `FREIGHTOS_SCHEMA_DRIFT` — result schema validation failed
  - `FREIGHTOS_AUTH_PROMPT` — paid-auth prompt encountered (per TDD §5.1)
- Transcript evidence: store run manifest path under `evidenceTranscriptPath` per L4 contract; cite `agent_session_logs.run_manifest_path` if available.
- T3 live: invoke via CDP browser at `http://localhost:9222`; T1 unit tests use cassette/replay fixtures.

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-freight_engine-003` | T3 | Freightos + SeaRates CDP skills both invoked and normalized into low/base/high GBP bands; no paid API account used (Freightos half lives here, SeaRates half in adapter bead) |
| `L5-T-freight_engine-008` | T1 | Freightos no quote/page drift recorded with provider-specific code and transcript evidence (Freightos half lives here, SeaRates half in adapter bead) |

## Tests to Pass

- **`L5-T-freight_engine-003`** (T3) — partial (Freightos half)
- **`L5-T-freight_engine-008`** (T1) — partial (Freightos half)

## Acceptance Criteria

- [ ] `freightos-skill-adapter.ts` exists with the `FreightosAdapter`-style API.
- [ ] Output normalized to `FreightEstimate` shape from `freight-types.ts`.
- [ ] 5 Freightos failure codes emitted on the documented conditions.
- [ ] No paid API path; CDP-only invocation.
- [ ] Test function names contain `L5-T-freight_engine-003` / `008` verbatim.
- [ ] `bun run test` passes for adapter tests.

## Dependencies

- **Blocked by:** `l5-p3-freightos-prerequisite-check`, `l5-p3-logistics-types-mode-selector`.
- **Blocks:** `l5-p3-dual-provider-disagreement`, `l5-p3-freight-boundary-defaults`, `l5-p3-freight-source-health-gate`.
- **Phase:** 3.

---

### l5-p3-searates-adapter

- id: `testproject1-hn05.3.4`
- type: task
- priority: P0
- labels: adapters,freight,l5,l5-phase-3,layer-5
- parent: `testproject1-hn05.3`
- blocked by: `testproject1-hn05.3.2`

#### Description

# l5-p3-searates-adapter

## SOURCE OF TRUTH WARNING

- **Spec**: §2.2.2, §2.2.4 (PRIMARY)
- **TDD**: §8.5 `L5-T-freight_engine-003,008` (PRIMARY)
- **Strategy**: Phase 3 (parallel with Freightos adapter after types land)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Wraps the existing `skills/landed-cost/searates-freight-estimate/` CDP browser skill (already mature: 19381-byte SKILL.md, 1325-byte result.schema.json). Normalizes output to standard `FreightEstimate`. Records SeaRates-specific failure codes on no-quote, schema drift. Extends the existing skill where needed for dual-source disagreement signaling without breaking existing callers.

## Files to Create/Modify

- **Create:** `packages/intelligence/src/logistics/searates-skill-adapter.ts`
- **Modify:** `packages/intelligence/src/logistics/freight-engine.test.ts` (add SeaRates halves of `L5-T-freight_engine-003,008`)
- **Possibly modify:** `skills/landed-cost/searates-freight-estimate/SKILL.md` if extension is required for disagreement signaling (operator-built; tread carefully — only document the additional output field if needed)

## Spec Requirements (verbatim)

**L5-FR-2.2.2 — Freight estimates use Freightos and SeaRates CDP skills.**
- AC1: invokes both for dual-source cross-check.
- AC2: Large provider disagreement records `FREIGHT_MODE_LOW_CONFIDENCE` in `warnings` or `blockers`; §3.5 remains the 10-code daemon error-vocabulary extension listed there.

**L5-FR-2.2.4 — Freight error paths are source-specific.**
- AC2: SeaRates no quote, unsupported mode, or schema drift is recorded separately.

## Implementation Details

`searates-skill-adapter.ts` exports:

```ts
import type { FreightEstimate, ModeSelection } from "./freight-types.js";

export interface SearatesAdapter {
  estimate(opts: {
    mode: ModeSelection["mode"];
    originPort: string;
    destinationPort: string;
    weightKg: number;
    volumeM3: number;
    cassettePath?: string;
  }): Promise<FreightEstimate>;
}
```

Behavior mirrors Freightos adapter; SeaRates-specific failure codes:

- `SEARATES_NO_QUOTE`
- `SEARATES_UNSUPPORTED_MODE`
- `SEARATES_SCHEMA_DRIFT`
- `SEARATES_PERSISTENT_CDP_REQUIRED` (per existing bead `testproject1-z7zz` "Document persistent CDP setup for Stage 11 SeaRates validation")
- `SEARATES_RATE_LIMITED`

Disagreement signaling extension: the existing SKILL.md MAY need a documented field signaling whether SeaRates' result agrees with a second provider — but the adapter level is the right home for disagreement detection, NOT the skill. So this adapter computes disagreement against a paired Freightos result if both are available. The actual disagreement logic lives in `l5-p3-dual-provider-disagreement`.

Tests to add:

- **`L5-T-freight_engine-003`** (T3) — SeaRates half
- **`L5-T-freight_engine-008`** (T1) — SeaRates half

## Tests to Pass

- **`L5-T-freight_engine-003`** (T3) — SeaRates half
- **`L5-T-freight_engine-008`** (T1) — SeaRates half

## Acceptance Criteria

- [ ] `searates-skill-adapter.ts` exists with the `SearatesAdapter`-style API.
- [ ] Output normalized to `FreightEstimate`.
- [ ] 5 SeaRates failure codes emitted on the documented conditions.
- [ ] No modifications to `searates-freight-estimate/SKILL.md` UNLESS strictly required; if modified, the change is additive (new nullable fields with default behavior).
- [ ] Tests pass `bun run test`.

## Dependencies

- **Blocked by:** `l5-p3-logistics-types-mode-selector` (Phase 1 done already implied).
- **Blocks:** `l5-p3-dual-provider-disagreement`, `l5-p3-freight-boundary-defaults`, `l5-p3-freight-source-health-gate`.
- **Phase:** 3.

---

### l5-p3-dual-provider-disagreement

- id: `testproject1-hn05.3.5`
- type: task
- priority: P0
- labels: freight,l5,l5-phase-3,layer-5
- parent: `testproject1-hn05.3`
- blocked by: `testproject1-hn05.3.4`, `testproject1-hn05.3.3`

#### Description

# l5-p3-dual-provider-disagreement

## SOURCE OF TRUTH WARNING

- **Spec**: §2.2.2 AC2, §2.2.4 AC3, §3.3.17, §3.5.10 (PRIMARY)
- **TDD**: §8.5 `L5-T-freight_engine-004,005`, §7.3 (`FREIGHT_PROVIDER_DISAGREEMENT_THRESHOLD_BPS = 2500`), §11.7 (Freight Discrepancy Contract) (PRIMARY)
- **Strategy**: Phase 3 (after both adapters)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

When Freightos and SeaRates both return quotes, compares the two `base_gbp_pence` values. If the relative difference exceeds the threshold constant, emits `FREIGHT_MODE_LOW_CONFIDENCE` warning AND selects the conservative (higher) base band — never averages. When both providers fail, emits `FREIGHT_ESTIMATE_FAILED` and preserves the manual-estimate requirement.

## Files to Create/Modify

- **Modify:** `packages/intelligence/src/logistics/freight-engine.ts` (this is part of the freight-engine module; full file ownership is in `l5-p3-freight-boundary-defaults`; this bead contributes the disagreement section)
- **Modify:** `packages/intelligence/src/logistics/freight-engine.test.ts` (add `L5-T-freight_engine-004,005`)

## Spec Requirements (verbatim)

**L5-FR-2.2.2 — AC2:** Large provider disagreement records `FREIGHT_MODE_LOW_CONFIDENCE` in `warnings` or `blockers`.

**L5-FR-2.2.4 — AC3:** If neither provider can quote, emit `FREIGHT_ESTIMATE_FAILED`, preserve a manual-estimate requirement, and do not fabricate provider data.

**L5-FR-3.3.17 — Scenario: Freightos and SeaRates disagreement and dual failure.**
- AC1: Provider disagreement records `FREIGHT_MODE_LOW_CONFIDENCE`; dual provider failure emits `FREIGHT_ESTIMATE_FAILED` and preserves the manual-estimate requirement.

**L5-FR-3.5.10 — Error code `FREIGHT_ESTIMATE_FAILED`.**
- AC1: Trigger is neither Freightos nor SeaRates can quote; action requires manual freight estimate and records warning/blocker.

## Implementation Details

Threshold constant (TDD §7.3):

```ts
export const FREIGHT_PROVIDER_DISAGREEMENT_THRESHOLD_BPS = 2500;
```

Disagreement detection logic:

```ts
import type { FreightEstimate } from "./freight-types.js";

export type DualProviderOutcome =
  | { kind: "agreed"; selected: FreightEstimate; otherProvider: FreightEstimate }
  | { kind: "disagreement"; selected: FreightEstimate; otherProvider: FreightEstimate; deltaBps: number; warning: "FREIGHT_MODE_LOW_CONFIDENCE" }
  | { kind: "single_provider"; selected: FreightEstimate; missingProvider: "freightos" | "searates" }
  | { kind: "dual_failure"; failure: "FREIGHT_ESTIMATE_FAILED"; manualEstimateRequired: true };

export function reconcile(freightos: FreightEstimate | null, searates: FreightEstimate | null): DualProviderOutcome {
  const freightosOk = freightos && freightos.baseGbpPence !== null;
  const searatesOk = searates && searates.baseGbpPence !== null;
  if (!freightosOk && !searatesOk) {
    return { kind: "dual_failure", failure: "FREIGHT_ESTIMATE_FAILED", manualEstimateRequired: true };
  }
  if (freightosOk && !searatesOk) {
    return { kind: "single_provider", selected: freightos!, missingProvider: "searates" };
  }
  if (!freightosOk && searatesOk) {
    return { kind: "single_provider", selected: searates!, missingProvider: "freightos" };
  }
  // Both ok
  const f = freightos!.baseGbpPence!;
  const s = searates!.baseGbpPence!;
  const max = Math.max(f, s);
  const min = Math.min(f, s);
  const deltaBps = Math.round(((max - min) / min) * 10000);
  if (deltaBps > FREIGHT_PROVIDER_DISAGREEMENT_THRESHOLD_BPS) {
    const selected = max === f ? freightos! : searates!;
    const other = max === f ? searates! : freightos!;
    return { kind: "disagreement", selected, otherProvider: other, deltaBps, warning: "FREIGHT_MODE_LOW_CONFIDENCE" };
  }
  // Agreed — select conservative (higher) for consistency
  return { kind: "agreed", selected: max === f ? freightos! : searates!, otherProvider: max === f ? searates! : freightos! };
}
```

Discrepancy fixture from TDD §11.7 (`L5-T-freight_engine-004`): freightos_base=120000, searates_base=185000 → relative delta `(185000-120000)/120000*10000 = 5417 bps`, exceeds 2500 → emit `FREIGHT_MODE_LOW_CONFIDENCE`, select 185000 (conservative).

Dual-failure case: `selected_base_gbp_pence = null`, `manual_estimate_required = true`, emit `FREIGHT_ESTIMATE_FAILED`.

## Tests to Pass

- **`L5-T-freight_engine-004`** (T1, discrepancy fixture): Freightos 120,000 pence + SeaRates 185,000 pence exceed threshold, emit `FREIGHT_MODE_LOW_CONFIDENCE`, select conservative higher base
- **`L5-T-freight_engine-005`** (T1, dual failure fixture): both providers failing emits `FREIGHT_ESTIMATE_FAILED`, preserves manual-estimate requirement, no fabricated data

## Acceptance Criteria

- [ ] `reconcile()` function exists in `freight-engine.ts` exporting the typed outcome.
- [ ] `FREIGHT_PROVIDER_DISAGREEMENT_THRESHOLD_BPS = 2500` named constant exported.
- [ ] Discrepancy fixture math matches TDD §11.7 exactly.
- [ ] Dual-failure path emits `FREIGHT_ESTIMATE_FAILED` (daemon enum from Phase 1) without fabricating quote data.
- [ ] Conservative-selection rule: when disagreed, picks the higher of the two `baseGbpPence` values.
- [ ] Both test IDs pass.

## Dependencies

- **Blocked by:** `l5-p3-freightos-adapter`, `l5-p3-searates-adapter`.
- **Blocks:** `l5-p3-freight-boundary-defaults`, `l5-p3-freight-source-health-gate`.
- **Phase:** 3.

---

### l5-p3-freight-boundary-defaults

- id: `testproject1-hn05.3.6`
- type: task
- priority: P0
- labels: freight,l5,l5-phase-3,layer-5
- parent: `testproject1-hn05.3`
- blocked by: `testproject1-hn05.3.5`, `testproject1-hn05.3.4`, `testproject1-hn05.3.3`, `testproject1-hn05.3.2`

#### Description

# l5-p3-freight-boundary-defaults

## SOURCE OF TRUTH WARNING

- **Spec**: §1.12.1, §2.2.3, §3.6.1, §3.6.2 (PRIMARY)
- **TDD**: §8.5 `L5-T-freight_engine-006,007`, `L5-T-customs_classification-017` (boundary scenario overlap) (PRIMARY)
- **Strategy**: Phase 3 (boundary defaults + freight-engine.ts main module)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Owns `packages/intelligence/src/logistics/freight-engine.ts` — the unified freight orchestrator. Implements Incoterm boundary scenarios (EXW/FOB/CIF) when Incoterm is missing, dual-packing scenarios (palletized vs loose cartons) when packaging is unknown, and selects the highest-cost feasible boundary per the open-question §3.6.2 runtime contract. Conservative default: never relaxing assumptions; surfacing unlockable margin instead.

## Files to Create/Modify

- **Create:** `packages/intelligence/src/logistics/freight-engine.ts`
- **Modify:** `packages/intelligence/src/logistics/freight-engine.test.ts` (add `L5-T-freight_engine-006,007` — overlap with `L5-T-customs_classification-017`)

## Spec Requirements (verbatim)

**L5-FR-2.2.3 — Freight conservative defaults are explicit.**
- AC1: Unknown packaging runs dual scenarios such as palletized versus loose cartons.
- AC2: Missing Incoterm runs EXW/FOB/CIF boundary scenarios.
- AC3: If neither provider can quote, emit `FREIGHT_ESTIMATE_FAILED`, preserve a manual-estimate requirement, and do not fabricate provider data.
- AC4: While product-family logistics heuristics remain uncalibrated, unknown item class or missing dimensional data uses the highest-cost feasible boundary scenario and records the missing logistics facts.

**L5-FR-1.12.1 (Conservative defaults table row "Incoterm"):** Boundary scenarios when no stated Incoterm; "Supplementary units → Abstain on duty for that component" (this last belongs to customs, not freight, but the freight Incoterm scenario is in scope).

**L5-FR-3.6.1, §3.6.2 — Product-family logistics open question:** runtime behavior while open = "highest-cost feasible freight boundary when category-specific heuristics are not calibrated; block automated completion if weight/volume evidence is insufficient."

## Implementation Details

`freight-engine.ts` orchestrates:

```ts
import { selectMode } from "./mode-selector.js";
import { reconcile } from "./freight-engine.js"; // self-export, re-exporting reconcile from disagreement bead
import type { FreightEstimate, ModeSelectionFacts, PackingScenario, ShippingMode } from "./freight-types.js";
import type { FreightosAdapter } from "./freightos-skill-adapter.js";
import type { SearatesAdapter } from "./searates-skill-adapter.js";

export interface FreightEngineInputs {
  facts: ModeSelectionFacts;
  incoterm: "EXW" | "FOB" | "CIF" | "DDP" | "unknown";
  packagingState: "palletized" | "loose_cartons" | "unknown";
  originPort: string;
  destinationPort: string;
}

export interface FreightEngineOutputs {
  selectedMode: ShippingMode;
  selectedEstimate: FreightEstimate | null;          // null when dual_failure
  boundaryScenariosRun: Array<{ scenario: string; estimate: FreightEstimate | null }>;
  outcomeKind: "agreed" | "disagreement" | "single_provider" | "dual_failure";
  warningsAndBlockers: string[];                      // e.g., FREIGHT_MODE_LOW_CONFIDENCE, FREIGHT_ESTIMATE_FAILED
  manualEstimateRequired: boolean;
  unlockableMarginNote?: string;
}

export class FreightEngine {
  constructor(freightos: FreightosAdapter, searates: SearatesAdapter) {}
  async estimate(inputs: FreightEngineInputs): Promise<FreightEngineOutputs> { /* ... */ }
}
```

Boundary scenario logic:

- `incoterm = "unknown"` → run `EXW`, `FOB`, `CIF` scenarios. Each scenario yields a freight estimate via `reconcile(freightos, searates)`. Select the highest-cost feasible scenario.
- `packagingState = "unknown"` → run `palletized` and `loose_cartons` scenarios. Select the higher-cost-feasible boundary.
- Missing dimensional data (weight or volume null) → emit blocker `FREIGHT_DIMENSIONAL_DATA_MISSING` AND set `manualEstimateRequired = true`. (This is NOT a new daemon enum variant; it's a string in `warningsAndBlockers` — Phase 1's `FREIGHT_ESTIMATE_FAILED` covers the daemon vocabulary.)
- Conservative selection: never average across scenarios; pick the worst-case (highest cost).

Cross-cutting with customs: `L5-T-customs_classification-017` ("Incoterm default fixture: missing Incoterm runs exactly `["EXW","FOB","CIF"]` boundary scenarios") is overlap. This bead owns the freight half (running the 3 scenarios + selecting boundary); the customs half (storing required logistics evidence + selecting highest feasible landed-cost branch) lives in `l5-p4-conservative-defaults-evidence-leverage`. Both must agree on the same test fixture.

Tests to add (TDD §8.5):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-freight_engine-006` | T1 | unknown packaging runs palletized + loose-carton scenarios; selects highest-cost feasible boundary until calibrated |
| `L5-T-freight_engine-007` | T1 | missing Incoterm runs EXW/FOB/CIF boundary scenarios and records missing logistics facts |

## Tests to Pass

- **`L5-T-freight_engine-006`** (T1)
- **`L5-T-freight_engine-007`** (T1)

## Acceptance Criteria

- [ ] `freight-engine.ts` exists with the unified `FreightEngine` orchestrator.
- [ ] EXW/FOB/CIF scenarios run exactly when `incoterm = "unknown"`.
- [ ] Palletized + loose-cartons scenarios run exactly when `packagingState = "unknown"`.
- [ ] Conservative selection: highest-cost feasible boundary, no averaging.
- [ ] Missing dimensional data → manual-estimate requirement + blocker recorded.
- [ ] `L5-T-customs_classification-017` agrees on the same EXW/FOB/CIF set with this bead's freight half.
- [ ] Both test IDs pass.

## Dependencies

- **Blocked by:** `l5-p3-logistics-types-mode-selector`, `l5-p3-freightos-adapter`, `l5-p3-searates-adapter`, `l5-p3-dual-provider-disagreement`.
- **Blocks:** `l5-p3-freight-source-health-gate`.
- **Phase:** 3.

---

### l5-p3-freight-source-health-gate

- id: `testproject1-hn05.3.7`
- type: task
- priority: P0
- labels: l5,l5-phase-3,layer-5,review-gate
- parent: `testproject1-hn05.3`
- blocked by: `testproject1-hn05.3.6`, `testproject1-hn05.3.5`, `testproject1-hn05.3.4`, `testproject1-hn05.3.3`, `testproject1-hn05.3.2`, `testproject1-hn05.3.1`, `testproject1-hn05.1.5`

#### Description

# l5-p3-freight-source-health-gate

## SOURCE OF TRUTH WARNING

- **Spec**: §2.2 (full), §3.1.10, §3.5.10, §3.7.4 (PRIMARY)
- **TDD**: `L5-T-freight_engine-001..008,015,016`, daemon target `customs_fx_freight_engine.rs` (PRIMARY)
- **Strategy**: Phase 3 gate

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Phase 3 review gate. Verifies the full freight engine works end-to-end, source-health canary cron is bound and exercises Freightos/SeaRates readiness, and the daemon integration target asserts provider discrepancy/failure persistence into `landed_cost_estimates`.

## Files to Inspect/Modify

- All Phase 3 task bead outputs.
- **Extend:** `crates/daemon/tests/customs_fx_freight_engine.rs` (already extended by Phase 2 with FX tests) — add freight provider discrepancy + failure persistence test functions.

## Gate Criteria (strategy §Phase 3)

1. `freight-engine.test.ts` covers `L5-T-freight_engine-001..008,015,016`.
2. Dual-source disagreement emits low-confidence warning path under fixture data.
3. Dual-provider failure emits `FREIGHT_ESTIMATE_FAILED` and preserves manual-estimate requirement.
4. Missing logistics assumptions follow §3.6 present-tense runtime behavior.
5. Freightos and SeaRates stay CDP/browser skill paths with no paid API fallback.

## Implementation Details

Daemon-side integration tests required (in `customs_fx_freight_engine.rs`):

```rust
#[test]
fn l5_t_freight_engine_016_marketplace_source_health_cron_exists() {
    // Inspect cron registry for MARKETPLACE_SOURCE_HEALTH_CANARY_CRON
    // Assert: source_id == "l5-commercial-intelligence"
    // Assert: skill_id targets source-health/l5-marketplace-commercial-intelligence
    // Assert: covers Alibaba, 1688, Amazon, PCS, SeaRates, Freightos, TARIC IDs
}

#[test]
fn l5_t_freight_engine_015_prerequisite_skill_blocker_persists() {
    // Verify external dependency fixture: missing canary/golden/rubric → stable-production blocker
}
```

Verification checklist:

```bash
bun test packages/intelligence/src/logistics/freight-engine.test.ts
for i in 001 002 003 004 005 006 007 008 015 016; do
  bun test packages/intelligence/src/logistics/freight-engine.test.ts | rg -q "L5-T-freight_engine-$i" || exit 1
done

cargo test --test customs_fx_freight_engine

# Source-health canary cron registered
rg -q "MARKETPLACE_SOURCE_HEALTH_CANARY_CRON" crates/daemon/src/scheduler/cron.rs

# No paid API path
! rg -q "(FREIGHTOS|SEARATES)_API_KEY" packages/intelligence/src/logistics/
```

## Tests to Pass

- **`L5-T-freight_engine-001..008`** (8 IDs)
- **`L5-T-freight_engine-015`** (T2 prerequisite-skill blocker)
- **`L5-T-freight_engine-016`** (T2 source-health cron)

## Acceptance Criteria

- [ ] All 10 listed test IDs pass.
- [ ] `customs_fx_freight_engine.rs` has freight-side tests for `015,016` and any persistence assertions.
- [ ] Source-health cron covers Freightos/SeaRates/TARIC readiness without paid API auth checks.
- [ ] No paid API fallback path in `packages/intelligence/src/logistics/`.
- [ ] Phase 3 roll-back conditions NOT triggered.
- [ ] `bun run build && bun run test` exits 0.

## Dependencies

- **Blocked by:** `l5-p3-freight-boundary-defaults`, `l5-p1-cron-definitions`, all other Phase 3 task beads.
- **Blocks:** `l5-p6-mainrs-feature-flag-shadow`, `l5-p6-final-dod-readiness-gate`.
- **Phase:** 3 (final).

---

## Phase 4 Task Beads

### l5-p4-hmrc-fpo-taric-discovery

- id: `testproject1-hn05.4.1`
- type: task
- priority: P0
- labels: customs,fpo,l5,l5-phase-4,layer-5,taric
- parent: `testproject1-hn05.4`
- blocked by: `testproject1-hn05.1.8`

#### Description

# l5-p4-hmrc-fpo-taric-discovery

## SOURCE OF TRUTH WARNING

- **Spec**: §1.4.2, §1.4.3, §1.4.5, §3.1.4, §3.7.1, §3.9.2 (PRIMARY)
- **TDD**: §8.1 `L5-T-hmrc_fpo-002,003,007,008,011,012` (PRIMARY)
- **Strategy**: Phase 4 (FPO/TARIC endpoint investigation first; TARIC prerequisite gates this)
- **Bootstrap**: §4.1 (account state — no key needed), §5 (preflight)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`. **MUST verify `skills/landed-cost/taric-cross-check/{SKILL.md, result.schema.json, checkpoint.schema.json}` all exist before claiming** (§3.9.2 AC2).

## Context

Two concerns merged into one bead because both involve external authority resolution:

1. **HMRC FPO endpoint discovery** — `l5-customs-hmrc-fpo-endpoint-discovery` investigation: determine whether `/api/search`, `/api/search_suggestions`, public FPO endpoint, or an operator decision about Developer Portal registration is needed. HMRC Trade Tariff commodity APIs are already public and require no Developer Portal key (per §1.4.2 AC3).
2. **TARIC cross-check adapter** — wraps the operator-built `skills/landed-cost/taric-cross-check/` skill. TARIC disagreement is advisory only; HMRC remains the UK legal authority for UK national measures, VAT additional codes, UK quotas, and UK trade remedies.

## Files to Create/Modify

- **Create:** initial scaffolding inside `packages/intelligence/src/customs/multi-source-recall.ts` for the FPO/search-suggestion source family + TARIC source family. The full multi-source recall module is owned by `l5-p4-hmrc-multi-family-expansion`; this bead contributes the FPO/TARIC source adapters within it.
- **Create:** `packages/intelligence/src/customs/hmrc-fpo.test.ts` (test file for the FPO/search-suggestion subset of TDD §8.1).
- **Record investigation outcome:** create or update `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/hmrc-fpo-endpoint-decision.md` documenting which endpoint path was selected.

## Spec Requirements (verbatim)

**L5-FR-1.4.2 — FPO agreement increases confidence but FPO alone is not evidence.**
- AC1: FPO/search-suggestion output alone never unlocks auto-classification.
- AC2: Agreement must be evaluated through independent source families and slot-level compatibility against the tariff hierarchy.
- AC3: Step 03 TDD creates investigation bead `l5-customs-hmrc-fpo-endpoint-discovery` to determine whether `/api/search`, `/api/search_suggestions`, a public FPO endpoint, or an operator decision about Developer Portal registration is required. HMRC Trade Tariff commodity APIs are already public and require no Developer Portal key.

**L5-FR-1.4.3 — HMRC remains the UK legal authority.**
- AC1: EU TARIC cross-checks and any advisory recall tools never overrule HMRC on UK national measures, VAT additional codes, UK quotas, or UK trade remedies.
- AC2: A cross-check disagreement is recorded as evidence divergence and cannot overwrite HMRC UK measure data.

**L5-FR-1.4.5 — Recall source error paths are explicit.**
- AC1: HMRC Trade Tariff rate-limit, downtime, and schema drift produce a blocked or stale classification path, not guessed tariff data.
- AC2: FPO/search-suggestion unavailable or malformed responses reuse existing daemon code `FPO_ACCESS_UNAVAILABLE` and continue with remaining sources.
- AC3: EU TARIC disagreement is advisory evidence divergence; HS Ping is declined for L5 and not required for green status.

**L5-FR-3.1.4 — DoD 4: HMRC FPO/search-suggestion endpoint investigation completed.**
- AC1: Step 03 TDD records the selected endpoint path and tests implemented recall behavior.

**L5-FR-3.7.1 — Risk: HMRC FPO/search-suggestion endpoint is unavailable or not public.**
- AC1: Mitigation is endpoint discovery bead choosing public Trade Tariff search, public FPO, or operator decision path; FPO is not a hard dependency.

**L5-FR-3.9.2 — Operator-built prerequisites gate bead execution.**
- AC2: Phase 4 Customs recall beads cannot start until `skills/landed-cost/taric-cross-check/` exists with `SKILL.md`, `result.schema.json`, and `checkpoint.schema.json`.

## Implementation Details

Source families implemented here (§1.4.1):

1. `hmrc_fpo_or_search_suggestion` — `/api/search`, `/api/search_suggestions`, OR public FPO (decision recorded by this bead).
2. (owned by sibling Phase 4 beads: `cpv_chapter_prior`, `rule_based_chapter_hint`, `hmrc_tariff_keyword_search`, `atr_codebook_reuse`)

TARIC adapter:

```ts
import type { /* candidate, divergence types */ } from "./types.js";

export interface TaricCrossCheck {
  // Invoke the operator-built skill via standard skill runner
  lookup(opts: {
    code: string;     // HS6, CN8, or UK10 (TARIC supports HS6/CN8)
    cassettePath?: string;
  }): Promise<TaricLookupResult>;
}

export interface TaricLookupResult {
  status: "ok" | "unavailable" | "schema_drift";
  measures: TaricMeasure[];           // EU measures, NOT UK measures
  isAdvisoryOnly: true;                // constant — TARIC is never authoritative for UK national rules
}
```

FPO endpoint decision matrix (this bead records ONE outcome):

| Endpoint | Public | Authority | L5 Decision |
|----------|--------|-----------|-------------|
| `https://www.trade-tariff.service.gov.uk/api/search?q=...` | yes | tariff_keyword_search family | **Primary L5 recall source** (separate adapter in `l5-p4-hmrc-multi-family-expansion`) |
| `https://www.trade-tariff.service.gov.uk/api/search_suggestions?q=...` | yes (if exists) | FPO-like relevance | **Use if reachable**; not legal evidence |
| Public FPO at separate hostname | unknown | retrieval-relevance | **Test live** |
| Developer Portal FPO | requires registration | retrieval-relevance | **Decline** — bootstrap forbids account creation |

The investigation outcome must be recorded as a 1-page Markdown file under `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/hmrc-fpo-endpoint-decision.md` with: endpoint chosen, sample successful response cassette path, fallback behavior if endpoint becomes unavailable.

Tests to add (TDD §8.1, FPO/TARIC subset):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-hmrc_fpo-002` | T1 | parses `/api/search_suggestions` or selected public replacement into candidates without treating suggestions as legal evidence |
| `L5-T-hmrc_fpo-003` | T3 | live HMRC probe records endpoint decision; no procurement action created |
| `L5-T-hmrc_fpo-007` | T1 | TARIC support disagrees on UK national measure; HMRC remains authoritative; TARIC stored as evidence divergence only |
| `L5-T-hmrc_fpo-008` | T1 | malformed FPO cassette: emits existing `FPO_ACCESS_UNAVAILABLE`, continues remaining source families, no duplicate `FPO_UNAVAILABLE` |
| `L5-T-hmrc_fpo-011` | T3 | unauthenticated HMRC Trade Tariff public API returns commodity/search body shape; rate-limit/downtime produces blocked or stale path, never guessed data |
| `L5-T-hmrc_fpo-012` | T1 | source registry fixture: HS Ping declined; 4 non-FPO recall families + HMRC public search sufficient for green status if FPO unavailable |

## Tests to Pass

- **`L5-T-hmrc_fpo-002`** (T1)
- **`L5-T-hmrc_fpo-003`** (T3)
- **`L5-T-hmrc_fpo-007`** (T1)
- **`L5-T-hmrc_fpo-008`** (T1)
- **`L5-T-hmrc_fpo-011`** (T3)
- **`L5-T-hmrc_fpo-012`** (T1)

## Acceptance Criteria

- [ ] TARIC prerequisite skill files all exist before this bead claimed (otherwise BLOCKED).
- [ ] FPO endpoint decision recorded in `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/hmrc-fpo-endpoint-decision.md`.
- [ ] FPO/search-suggestion adapter emits `FPO_ACCESS_UNAVAILABLE` (existing daemon enum) on missing/malformed; never adds duplicate `FPO_UNAVAILABLE`.
- [ ] TARIC adapter records disagreement as evidence divergence, never overrules HMRC UK measures.
- [ ] All 6 listed test IDs pass.
- [ ] No Developer Portal registration step recorded anywhere.

## Dependencies

- **Blocked by:** `l5-p1-scope-preservation-gate`, TARIC prerequisite skill files present.
- **Blocks:** `l5-p4-description-quality-recall`, `l5-p4-hmrc-multi-family-expansion`.
- **Phase:** 4.

---

### l5-p4-description-quality-recall

- id: `testproject1-hn05.4.2`
- type: task
- priority: P0
- labels: customs,l5,l5-phase-4,layer-5
- parent: `testproject1-hn05.4`
- blocked by: `testproject1-hn05.4.1`

#### Description

# l5-p4-description-quality-recall

## SOURCE OF TRUTH WARNING

- **Spec**: §1.4.4 (PRIMARY)
- **TDD**: `L5-T-hmrc_fpo-009,010` (PRIMARY)
- **Strategy**: Phase 4 (after FPO/TARIC discovery)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Owns `packages/intelligence/src/customs/description-quality.ts` (one of the 6 customs package files). Preprocessing before recall: rejects bundles, service descriptions, and generic nouns without required archetype facts. Emits archetype-keyed missing-facts requests. Bundle splitter decomposes multi-item descriptions only when safe; otherwise marks `mixed_manual` or GIR 3 candidate. Attribute extraction captures head noun, material, principal function, sector, sterility/medical status, power/size, pack form.

## Files to Create/Modify

- **Create:** `packages/intelligence/src/customs/description-quality.ts`
- **Create:** `packages/intelligence/src/customs/customs-classification.test.ts` (file header — tests added by later customs beads; this bead contributes preprocessing tests via `hmrc-fpo.test.ts`)
- **Modify:** `packages/intelligence/src/customs/hmrc-fpo.test.ts` (add `L5-T-hmrc_fpo-009,010`)

## Spec Requirements (verbatim)

**L5-FR-1.4.4 — Preprocessing before recall rejects low-quality descriptions and extracts classification facts.**
- AC1: Description-quality gate rejects bundles, service descriptions, and generic nouns without required archetype facts, and emits archetype-keyed missing-facts requests.
- AC2: Bundle splitter decomposes multi-item descriptions when safe and marks mixed/manual or GIR 3 candidate when decomposition is unsafe.
- AC3: Attribute extraction captures head noun, material, principal function, sector, sterility/medical status, power/size, and pack form.
- AC4: Multi-query recall runs normalized variants across all available recall sources.

## Implementation Details

`description-quality.ts` exports:

```ts
export interface RawDescription {
  text: string;
  cpvCode?: string;
  lotMetadata?: Record<string, unknown>;
}

export interface ExtractedAttributes {
  headNoun: string;
  material?: string;
  principalFunction?: string;
  sector?: string;
  sterilityMedical?: "sterile" | "non_sterile" | "medical_device" | "n_a";
  powerSize?: { power_w?: number; size_mm?: number; capacity_l?: number };
  packForm?: "retail_pack" | "bulk" | "set" | "kit" | "unknown";
}

export type DescriptionQualityVerdict =
  | { kind: "accept"; attributes: ExtractedAttributes }
  | { kind: "reject_generic"; reason: string; missingArchetypeFacts: string[] }
  | { kind: "reject_service"; reason: string }
  | { kind: "reject_bundle_unsafe"; reason: string; markAs: "mixed_manual" | "gir_3_candidate" }
  | { kind: "split_safe"; subDescriptions: RawDescription[] };

export function gateDescriptionQuality(input: RawDescription): DescriptionQualityVerdict;

export function generateQueryWaves(attrs: ExtractedAttributes): Array<{ wave: number; strategy: string; query_en: string; query_zh?: string }>;
```

Behavior:

- Reject generic single-noun descriptions ("equipment", "items", "products") without enriching attributes.
- Reject service descriptions (e.g., "consulting services", "training") — these are not goods.
- Bundle detection: if input contains `;`, `,`, `+`, or "and" connectors between distinct head nouns AND each segment has its own attributes → split.
- Otherwise mark `gir_3_candidate` (retail set) or `mixed_manual` (unsafe).
- Multi-query recall (per AC4) generates normalized variants: original, synonyms, head-noun-only, head-noun+material, head-noun+function.

Tests to add (TDD §8.1):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-hmrc_fpo-009` | T1 | preprocessing fixture: low-quality generic descriptions, service descriptions, unsafe bundles rejected before recall with archetype-keyed missing facts |
| `L5-T-hmrc_fpo-010` | T1 | query-wave fixture: normalized variants run across all available source families and still count as one source family per method (TDD §11.2 recall agreement contract — multiple waves in one family count once) |

## Tests to Pass

- **`L5-T-hmrc_fpo-009`** (T1)
- **`L5-T-hmrc_fpo-010`** (T1)

## Acceptance Criteria

- [ ] `description-quality.ts` exists.
- [ ] Generic noun rejection + service description rejection both emit archetype-keyed missing-facts requests.
- [ ] Bundle splitter: safe splits return `split_safe`, unsafe ones return `mixed_manual` or `gir_3_candidate`.
- [ ] `generateQueryWaves` returns multiple normalized variants but per §11.2 these count as one source family per method.
- [ ] Both test IDs pass.

## Dependencies

- **Blocked by:** `l5-p4-hmrc-fpo-taric-discovery`.
- **Blocks:** `l5-p4-hmrc-multi-family-expansion`, `l5-p4-exporter-origin-resolution`.
- **Phase:** 4.

---

### l5-p4-hmrc-multi-family-expansion

- id: `testproject1-hn05.4.3`
- type: task
- priority: P0
- labels: customs,l5,l5-phase-4,layer-5
- parent: `testproject1-hn05.4`
- blocked by: `testproject1-hn05.4.2`, `testproject1-hn05.4.1`

#### Description

# l5-p4-hmrc-multi-family-expansion

## SOURCE OF TRUTH WARNING

- **Spec**: §1.4.1, §1.4.5 (PRIMARY)
- **TDD**: `L5-T-hmrc_fpo-001,004,005,006`, §11.2 (Recall Agreement Trace Contract) (PRIMARY)
- **Strategy**: Phase 4 (after description quality lands)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Owns `packages/intelligence/src/customs/multi-source-recall.ts` (one of the 6 customs package files). Implements the 5 parallel recall sources from §1.4.1, computes `candidateRecallAgreement` per §11.2, and exposes the result for downstream UK10 resolution. Each multi-query wave inside one source family counts as ONE source family per method (per §11.2).

## Files to Create/Modify

- **Create:** `packages/intelligence/src/customs/multi-source-recall.ts`
- **Modify:** `packages/intelligence/src/customs/hmrc-fpo.test.ts` (add `L5-T-hmrc_fpo-001,004,005,006`)

## Spec Requirements (verbatim)

**L5-FR-1.4.1 — Multi-source recall uses five parallel recall sources.**
- AC1: Every classification run attempts all applicable recall sources in the table below and records source availability, candidates, retrieval scores, and failure mode.
- AC2: If one recall source is unavailable, remaining sources still run and confidence is penalized rather than fabricated.
- AC3: Independent-source agreement means agreement across distinct source families with distinct data origin and retrieval logic; multiple query waves, translated terms, or endpoints inside one source family count once.

5 source families from §1.4.1 table:
1. **HMRC FPO or public search-suggestion endpoint** — `hmrc_fpo_or_search_suggestion` (covered by `l5-p4-hmrc-fpo-taric-discovery`)
2. **CPV-to-chapter priors** — `cpv_chapter_prior`
3. **Rule-based chapter hints** — `rule_based_chapter_hint`
4. **HMRC tariff keyword search** — `hmrc_tariff_keyword_search`
5. **ATR/codebook reuse** — `atr_codebook_reuse`

**L5-FR-1.4.5 — Recall source error paths are explicit.**
- AC1: HMRC Trade Tariff rate-limit, downtime, and schema drift produce a blocked or stale classification path, not guessed tariff data.
- AC2: FPO/search-suggestion unavailable or malformed responses reuse existing daemon code `FPO_ACCESS_UNAVAILABLE` and continue with remaining sources.

## Implementation Details

`multi-source-recall.ts` exports:

```ts
import type { ExtractedAttributes } from "./description-quality.js";

export type SourceFamily =
  | "hmrc_fpo_or_search_suggestion"
  | "cpv_chapter_prior"
  | "rule_based_chapter_hint"
  | "hmrc_tariff_keyword_search"
  | "atr_codebook_reuse";

export interface RecallCandidate {
  code: string;                     // HS6 or CN8
  level: "heading" | "subheading" | "cn8";
  retrievalScoreBps: number;
  sourceFamily: SourceFamily;
  rawEvidence?: unknown;             // raw source response for audit
}

export interface SourceFamilyResult {
  sourceFamily: SourceFamily;
  state: "agreed" | "diverged" | "unavailable";
  candidates: RecallCandidate[];
  scoreBps: number | null;
  failureMode?: "rate_limited" | "downtime" | "schema_drift" | "access_restricted" | "none";
}

export interface MultiSourceRecallResult {
  attributes: ExtractedAttributes;
  perSourceResults: SourceFamilyResult[]; // exactly 5 entries, one per family
  candidateRecallAgreement: Array<{ candidateCode: string; agreement: SourceFamilyResult[] }>;
  independentSourceFamiliesAvailable: number;  // count of state != "unavailable"
  independentSourceFamiliesAgreeing: number;   // count of state == "agreed" on top candidate
}

export class MultiSourceRecall {
  constructor(/* adapters: fpoAdapter, cpvPriorMap, ruleHintsTable, hmrcSearchAdapter, atrCodebookLookup */) {}
  async run(attributes: ExtractedAttributes, cpvCode?: string): Promise<MultiSourceRecallResult>;
}
```

Recall agreement trace per §11.2 (verbatim shape):

```json
{
  "candidateCode": "7318159590",
  "candidateRecallAgreement": [
    {"sourceFamily": "hmrc_fpo_or_search_suggestion", "state": "agreed", "scoreBps": 7100},
    {"sourceFamily": "cpv_chapter_prior", "state": "diverged", "scoreBps": 5200},
    {"sourceFamily": "rule_based_chapter_hint", "state": "agreed", "scoreBps": 8300},
    {"sourceFamily": "hmrc_tariff_keyword_search", "state": "agreed", "scoreBps": 7600},
    {"sourceFamily": "atr_codebook_reuse", "state": "unavailable", "scoreBps": null}
  ],
  "independentSourceFamiliesAvailable": 4,
  "independentSourceFamiliesAgreeing": 3
}
```

Trace persistence: result is stored in the `product_commodity_codes` table (one row per source-family per candidate) and in the `landed_cost_estimates.candidate_commodity_codes_json` rollup for the candidate-quote pair.

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-hmrc_fpo-001` | T1 | parses `/api/search?q=...` into HS6/CN8 candidates with source family `tariff_keyword_search`, retrieval scores, failure mode `none` |
| `L5-T-hmrc_fpo-004` | T1 | five-source recall bundle: all sources agree on same family; 5 source records present; confidence agreement-based, not FPO-only |
| `L5-T-hmrc_fpo-005` | T1 | disagreement bundle: 5 sources disagree; source-level divergence recorded; emit `CLASSIFICATION_AMBIGUOUS` when top HS6 families materially split |
| `L5-T-hmrc_fpo-006` | T1 | FPO-only bundle: FPO/search-suggestion candidate present but all other families absent; decision remains non-auto; records retrieval relevance only |

## Tests to Pass

- **`L5-T-hmrc_fpo-001`** (T1)
- **`L5-T-hmrc_fpo-004`** (T1)
- **`L5-T-hmrc_fpo-005`** (T1)
- **`L5-T-hmrc_fpo-006`** (T1)

## Acceptance Criteria

- [ ] `multi-source-recall.ts` exists with 5-family `SourceFamily` enum.
- [ ] `MultiSourceRecallResult.candidateRecallAgreement` shape matches §11.2 verbatim.
- [ ] Each source family is queryable in isolation; unavailability is tracked, not faked.
- [ ] FPO-only does NOT unlock auto-classification.
- [ ] `CLASSIFICATION_AMBIGUOUS` emission on top-HS6 disagreement.
- [ ] Multi-query waves inside one family count as ONE family per §1.4.1 AC3.
- [ ] All 4 listed test IDs pass.

## Dependencies

- **Blocked by:** `l5-p4-hmrc-fpo-taric-discovery`, `l5-p4-description-quality-recall`.
- **Blocks:** `l5-p4-tariff-graph-junction-writes`, `l5-p4-uk10-resolution-branching`.
- **Phase:** 4.

---

### l5-p4-tariff-graph-junction-writes

- id: `testproject1-hn05.4.4`
- type: task
- priority: P0
- labels: customs,l5,l5-phase-4,layer-5
- parent: `testproject1-hn05.4`
- blocked by: `testproject1-hn05.4.3`, `testproject1-hn05.1.1`

#### Description

# l5-p4-tariff-graph-junction-writes

## SOURCE OF TRUTH WARNING

- **Spec**: §1.5.3, §1.5.4, §1.3.2, §2.4.2 (PRIMARY)
- **TDD**: `L5-T-customs_classification-004,006`, §11.3 (Tariff Graph Expansion Contract) (PRIMARY)
- **Strategy**: Phase 4 (after recall + Phase 1 migration)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Implements the local HMRC tariff hierarchy cache as an indexed graph (per §1.5.4) and writes set-replaced rows into `product_commodity_codes` (Phase 1 migration). Runtime classification reads from this cache; live HMRC verification runs only on shortlisted candidates. Cache refresh follows §1.11 freshness vectors.

## Files to Create/Modify

- **Modify:** `packages/intelligence/src/customs/multi-source-recall.ts` (cache layer integration)
- **Create:** `packages/intelligence/src/customs/ten-digit-resolution.ts` (skeleton — full ownership in `l5-p4-uk10-resolution-branching`; this bead contributes the tariff graph + junction writes)
- **Modify:** `packages/intelligence/src/customs/customs-classification.test.ts` (add `L5-T-customs_classification-004,006`)

## Spec Requirements (verbatim)

**L5-FR-1.3.2 — L5 introduces the local SQLite `product_commodity_codes` junction.**
- AC3: Local rows use set-replacement semantics per candidate when classification reruns, matching the L5 classification lattice contract in `docs/strategy/layer-strategy.md` §10.

**L5-FR-1.5.3 — Expansion separates classification lattice from commercial scenario lattice.**
- AC1: Recall collects HS6/CN8 families; local cache expands descendants; local scoring shortlists top 3-5 non-dominated branches per family; external validation runs only on shortlisted candidates.
- AC2: Commercial variables (quotes, packing, Incoterms, exporter bases, quota states, FX) bind only after code-family pruning.

**L5-FR-1.5.4 — HMRC tariff data is locally cached as an indexed graph.**
- AC1: Runtime classification reads hierarchy, measures, supplementary-unit requirements, and additional-code requirements from local cache.
- AC2: Live HMRC verification runs for shortlisted candidates and cache refresh follows §1.11.

**L5-FR-2.4.2 — `product_commodity_codes` is actively populated from recall lattice output.**
- AC1: Each row links a `product_candidate` to a commodity code with recall source, agreement metadata, rank, score, and stage-run identity.
- AC2: Reruns atomically replace the candidate's prior commodity-code set.

## Implementation Details

Tariff graph fixture per TDD §11.3:

| Family | Initial Rank | Expected Result |
|--------|--------------|-----------------|
| `3923` packaging articles | 1 | expanded + shortlisted but rejected for carton material mismatch |
| `4819` paper/carton boxes | 2 | expanded + selected for packaging-cartons |
| `4823` other paper articles | 3 | expanded + retained as non-selected alternative |

Tariff graph data structures:

```ts
export interface TariffNode {
  code: string;                       // HS6/CN8/UK10
  parent: string | null;
  level: "chapter" | "heading" | "subheading" | "cn8" | "uk10";
  description: string;
  measures: TariffMeasure[];          // local cache of HMRC measures
  supplementaryUnitRequirements: string[];
  additionalCodeRequirements: AdditionalCodeRequirement[];
  scopeNotes: string[];
  exclusions: string[];
  freshness: {
    structureCheckedAt: string;       // ISO 8601 — §1.11 vector 1
    measuresFreshUntil: string;        // §1.11 vector 2
  };
}

export interface TariffGraph {
  getNode(code: string): TariffNode | null;
  descendants(code: string): TariffNode[];       // expand all UK10 descendants
  shortlistNonDominated(family: string, attrs: ExtractedAttributes, count: number): TariffNode[];
}
```

`product_commodity_codes` writer:

```ts
export async function persistRecallLattice(
  db: SqliteDb,
  opts: {
    productCandidateId: string;
    stageRunId: string;
    recallResult: MultiSourceRecallResult;
    expandedCandidates: Array<{ code: string; rank: number; sourceFamily: SourceFamily; retrievalScoreBps: number | null; agreementState: "agreed" | "diverged" | "unavailable" }>;
  }
): Promise<void> {
  // Set-replacement in a single SQLite transaction:
  // 1. DELETE FROM product_commodity_codes WHERE product_candidate_id = ? AND stage_run_id <> ?
  //    (preserves the new rows we're about to insert; deletes only prior cohorts)
  // 2. INSERT new rows, one per (candidate, sourceFamily) pair
  // 3. Compute content_hash, populate owner_id, row_version, last_local_seq via L4 outbox writer
}
```

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-customs_classification-004` | T2 | `L5StateDb` fixture: local HMRC tariff graph cache stores hierarchy, measures, supplementary units, additional-code requirements; live verification refreshes only shortlisted branches |
| `L5-T-customs_classification-006` | T2 | (also assigned to Phase 1 migration bead) `product_commodity_codes` mirrors L4 Supabase projection shape and set-replaces atomically under one `stage_run_id` |

## Tests to Pass

- **`L5-T-customs_classification-004`** (T2)
- **`L5-T-customs_classification-006`** (T2 — co-owned with `l5-p1-sqlite-product-commodity-codes` Phase 1; that bead owns the schema, this bead owns the WRITER semantics)

## Acceptance Criteria

- [ ] Tariff graph cache loads from a deterministic source (cassette or seeded SQLite table).
- [ ] Live HMRC verification only runs for shortlisted candidates, not the full descendant set.
- [ ] `persistRecallLattice` writes through L4 outbox (one transaction, `commit_group_id` covers all inserts).
- [ ] Set-replacement deletes prior stage_run cohorts before inserting new rows.
- [ ] `L5-T-customs_classification-004,006` both pass.

## Dependencies

- **Blocked by:** `l5-p1-sqlite-product-commodity-codes`, `l5-p4-hmrc-multi-family-expansion`.
- **Blocks:** `l5-p4-uk10-resolution-branching`, `l5-p4-classification-freshness`.
- **Phase:** 4.

---

### l5-p4-uk10-resolution-branching

- id: `testproject1-hn05.4.5`
- type: task
- priority: P0
- labels: customs,l5,l5-phase-4,layer-5
- parent: `testproject1-hn05.4`
- blocked by: `testproject1-hn05.4.4`

#### Description

# l5-p4-uk10-resolution-branching

## SOURCE OF TRUTH WARNING

- **Spec**: §1.5.1, §1.5.2, §1.5.5, §3.1.6 (PRIMARY)
- **TDD**: `L5-T-customs_classification-001,002,003,005` (PRIMARY)
- **Strategy**: Phase 4 (after tariff graph + junction writes)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Owns `packages/intelligence/src/customs/ten-digit-resolution.ts` (one of the 6 customs package files). NEVER greedy descent: expands ALL plausible HS6/CN8 families to ALL declarable UK10 descendants, then ranks. Operator presentation groups branches into family clusters (50+ descendant sets → cluster summary, not flat list).

## Files to Create/Modify

- **Modify (or create if not seeded by `l5-p4-tariff-graph-junction-writes`):** `packages/intelligence/src/customs/ten-digit-resolution.ts`
- **Modify:** `packages/intelligence/src/customs/customs-classification.test.ts` (add `L5-T-customs_classification-001,002,003,005`)

## Spec Requirements (verbatim)

**L5-FR-1.5.1 — Classification never uses greedy descent.**
- AC1: The engine expands all plausible HS6/CN8 families to all declarable UK10 descendants through HMRC tariff hierarchy data.
- AC2: A plausible runner-up family is not discarded until local scoring and shortlisting have run.

**L5-FR-1.5.2 — Descendant reranking uses slot and measure compatibility.**
- AC1: Extracted product attributes are scored against code descriptions, exclusions, scope notes, unit requirements, and measure profiles.
- AC2: `candidateRecallAgreement` records which recall sources agreed, diverged, or were unavailable for every candidate.

**L5-FR-1.5.5 — Operator presentation groups branches into family clusters.**
- AC1: UI/API output groups 50+ descendant sets by family label, count, conservative duty outcome, most-supported branch, blockers, and missing facts.
- AC2: Flat ungrouped UK10 lists are not an acceptable operator surface.

**L5-FR-3.1.6 — DoD 6: 10-digit resolution via HMRC Trade Tariff data produces UK-specific commodity codes.**
- AC1: UK10 output includes HMRC measure data and does not rely on non-UK authority for UK national measures.

## Implementation Details

```ts
import type { TariffGraph, TariffNode } from "./tariff-graph.js";
import type { ExtractedAttributes } from "./description-quality.js";
import type { MultiSourceRecallResult } from "./multi-source-recall.js";

export interface UkTenDigitCandidate {
  uk10Code: string;
  parentHs6: string;
  rank: number;
  scoreBps: number;
  slotCompatibility: {
    descriptionMatchBps: number;
    materialMatchBps: number;
    unitCompatibility: "ok" | "missing" | "incompatible";
    exclusionsTriggered: string[];
    scopeNoteFlags: string[];
  };
  candidateRecallAgreement: SourceFamilyResult[];
}

export interface FamilyClusterSummary {
  familyLabel: string;          // e.g., "4819 — Cartons, boxes, cases of paper or paperboard"
  count: number;                // descendant count
  conservativeDutyOutcome: { dutyPctBps: number; rationale: string };
  mostSupportedBranch: UkTenDigitCandidate;
  blockers: string[];
  missingFacts: string[];
}

export interface TenDigitResolutionResult {
  shortlisted: UkTenDigitCandidate[];           // top 3-5 non-dominated per family
  perFamilyClusters: FamilyClusterSummary[];
  greedyDescentDetected: false;                  // invariant — must always be false
}

export class TenDigitResolution {
  constructor(private graph: TariffGraph) {}
  resolve(attributes: ExtractedAttributes, recall: MultiSourceRecallResult): TenDigitResolutionResult;
}
```

Anti-greedy invariants:

- For every HS6 family returned by recall (NOT just top-ranked), expand all UK10 descendants.
- Score each UK10 against `attributes` independently.
- Shortlist top 3-5 non-dominated branches PER FAMILY (not 3-5 total).
- If `recall.perSourceResults[hmrc_fpo].candidates[0].code = "3923"` is top-ranked but slot compatibility for material rejects it, then `4819` (runner-up) must still be in the shortlisted output.

§11.3 fixture (verbatim assertion for `L5-T-customs_classification-001`):

```
{
  "candidate_commodity_codes_json": ["3923100000", "4819100000", "4823908580"],
  "selected_uk10": "4819100000",
  "rejection_reason_for_top_ranked": "material mismatch on 3923 (rigid plastic vs paperboard)"
}
```

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-customs_classification-001` | T1 | top-ranked `3923` rejected for material mismatch; runner-up `4819` selected; `candidate_commodity_codes_json` contains exactly sorted UK10 `["3923100000","4819100000","4823908580"]`; every plausible family expands before scoring |
| `L5-T-customs_classification-002` | T1 | descendant reranking uses slot compatibility, exclusions, supplementary units, measure profiles; `candidateRecallAgreement` for `4819100000` contains exactly 5 source-family states sorted by source family: `agreed, agreed, agreed, diverged, unavailable` |
| `L5-T-customs_classification-003` | T1 | recall lattice, classification lattice, commercial scenario lattice stay separate until top 3-5 non-dominated branches per family are shortlisted |
| `L5-T-customs_classification-005` | T1 | operator surface VM: 50+ descendants render as family clusters with label, count, conservative outcome, most-supported branch, blockers, missing facts; flat list output fails |

## Tests to Pass

- **`L5-T-customs_classification-001`** (T1)
- **`L5-T-customs_classification-002`** (T1)
- **`L5-T-customs_classification-003`** (T1)
- **`L5-T-customs_classification-005`** (T1)

## Acceptance Criteria

- [ ] `TenDigitResolution.resolve()` exists.
- [ ] Greedy descent disallowed by invariant.
- [ ] All plausible recall families expand to ALL UK10 descendants before scoring.
- [ ] Operator surface emits family clusters for 50+ descendants, not flat lists.
- [ ] Commercial variables (FX, freight, quote-role) DO NOT enter the classification lattice — strict separation.
- [ ] All 4 listed test IDs pass.

## Dependencies

- **Blocked by:** `l5-p4-tariff-graph-junction-writes`.
- **Blocks:** `l5-p4-measure-scanner-interactions`, `l5-p4-divergence-readiness-producers`.
- **Phase:** 4.

---

### l5-p4-exporter-origin-resolution

- id: `testproject1-hn05.4.6`
- type: task
- priority: P0
- labels: customs,l5,l5-phase-4,layer-5
- parent: `testproject1-hn05.4`
- blocked by: `testproject1-hn05.4.2`, `testproject1-hn05.1.3`

#### Description

# l5-p4-exporter-origin-resolution

## SOURCE OF TRUTH WARNING

- **Spec**: §1.6.1, §1.6.2 (PRIMARY)
- **TDD**: `L5-T-customs_classification-014`, `L5-T-measure_interaction-011,012,013` (PRIMARY)
- **Strategy**: Phase 4 (parallel after recall stable)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Owns `packages/intelligence/src/customs/exporter-resolution.ts` (one of the 6 customs package files). Trade-remedy exporter resolution defaults to residual rate when identity is unverified. Distributor / marketplace-seller / "factory direct" claims do not satisfy named-exporter proof.

## Files to Create/Modify

- **Create:** `packages/intelligence/src/customs/exporter-resolution.ts`
- **Modify:** `packages/intelligence/src/customs/customs-classification.test.ts` (add `L5-T-customs_classification-014`)
- **Modify:** `packages/intelligence/src/customs/measure-interaction.test.ts` (add `L5-T-measure_interaction-011,012,013`) — creates this file if not already present

## Spec Requirements (verbatim)

**L5-FR-1.6.1 — Trade-remedy exporter resolution defaults to residual rate when identity is unverified.**
- AC1: `exporterRateBasis` is one of `named_exporter`, `annex_exporter`, or `residual_default`.
- AC2: `residual_default` is used whenever exporter/manufacturer proof is absent or insufficient.
- AC3: `exporterIdentityConfidenceBps` records 0-10000 bps confidence based on supplier listing evidence quality.

**L5-FR-1.6.2 — Named-exporter relief requires manufacturer/exporter evidence.**
- AC1: Named or annex exporter rates require listing, invoice, certificate, or supplier-profile evidence tying the product to the eligible manufacturer/exporter.
- AC2: Distributor names, marketplace seller names, or vague "factory direct" claims do not satisfy named-exporter proof by themselves.
- AC3: Insufficient evidence emits `EXPORTER_UNRESOLVED`, applies the residual rate, and records the missing evidence in `confidence_breakdown.evidence_leverage`.

## Implementation Details

```ts
import type { SupplierQuote } from "../qualification/types.js"; // assume L4 contract

export type ExporterRateBasis = "named_exporter" | "annex_exporter" | "residual_default";

export interface ExporterResolutionInput {
  commodityCode: string;       // UK10
  countryOfOrigin: string;
  supplierQuotes: SupplierQuote[];
  evidenceArtifacts?: Array<{ kind: "invoice" | "certificate" | "supplier_profile" | "listing"; refPath: string }>;
  measuresAtCommodity: TariffMeasure[];   // from tariff graph cache
}

export interface ExporterResolutionResult {
  exporterRateBasis: ExporterRateBasis;
  exporterIdentityConfidenceBps: number;   // 0-10000
  selectedRatePctBps?: number;             // bps; null when no remedy active
  unlockableMarginBps?: number;            // delta if named/annex evidence provided
  errorCode?: "EXPORTER_UNRESOLVED";
  missingEvidence: string[];               // for confidence_breakdown.evidence_leverage
}

export function resolveExporter(input: ExporterResolutionInput): ExporterResolutionResult;
```

Decision matrix:

| Evidence | exporterRateBasis | exporterIdentityConfidenceBps | Rate Applied |
|----------|-------------------|------------------------------|---------------|
| Invoice + supplier profile naming the manufacturer; matches Annex list | `named_exporter` | ≥7500 | named rate |
| Listing + supplier profile naming the manufacturer; matches Annex list | `annex_exporter` | 5000-7500 | annex rate |
| Distributor name only | `residual_default` | ≤2500 | residual rate |
| Marketplace seller name only | `residual_default` | ≤2500 | residual rate |
| "Factory direct" claim only | `residual_default` | ≤2500 | residual rate |
| Nothing | `residual_default` | 0 | residual rate |

When `residual_default` is applied:
- Emit `EXPORTER_UNRESOLVED` (existing daemon enum from Phase 1).
- Compute `unlockableMarginBps` as the delta between named/annex rate and residual rate × customs value.
- Record missing evidence in `confidence_breakdown.evidence_leverage` array for the consumer (Phase 4 `l5-p4-conservative-defaults-evidence-leverage`).

Fixture (§8.3 `L5-T-measure_interaction-012`): aluminium extrusion, customs value GBP 10,000, named-exporter 0.0%, residual 35.1%. Missing proof → residual = 351,000 pence; `unlockable_margin_bps = 3510`.

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-customs_classification-014` | T1 | missing exporter proof: `exporter_rate_basis = "residual_default"`, `exporter_identity_confidence_bps <= 2500`, emit `EXPORTER_UNRESOLVED`, compute `unlockable_margin_bps = 3510` for aluminium fixture |
| `L5-T-measure_interaction-011` | T1 | active trade remedy with no manufacturer/exporter evidence defaults to `residual_default`, sets 0-10000 bps identity confidence |
| `L5-T-measure_interaction-012` | T1 | aluminium extrusion fixture: customs value GBP 10,000, named 0.0%, residual 35.1%; missing proof applies residual 351,000 pence, records named-exporter delta 3510 bps |
| `L5-T-measure_interaction-013` | T1 | distributor names, marketplace seller names, "factory direct" claims do NOT satisfy named-exporter proof |

## Tests to Pass

- **`L5-T-customs_classification-014`** (T1)
- **`L5-T-measure_interaction-011`** (T1)
- **`L5-T-measure_interaction-012`** (T1)
- **`L5-T-measure_interaction-013`** (T1)

## Acceptance Criteria

- [ ] `exporter-resolution.ts` exists.
- [ ] `exporterRateBasis` ∈ {`named_exporter`, `annex_exporter`, `residual_default`}.
- [ ] Aluminium fixture math: residual 35.1% × GBP 10,000 = 351,000 pence; unlockable delta 3510 bps.
- [ ] `EXPORTER_UNRESOLVED` emitted on residual default.
- [ ] Distributor/seller/factory-direct names rejected as proof.
- [ ] All 4 listed test IDs pass.

## Dependencies

- **Blocked by:** `l5-p4-description-quality-recall` (attributes extracted), `l5-p1-error-exhaustive-consumers` (EXPORTER_UNRESOLVED enum).
- **Blocks:** `l5-p4-measure-scanner-interactions`, `l5-p4-conservative-defaults-evidence-leverage`.
- **Phase:** 4.

---

### l5-p4-measure-scanner-interactions

- id: `testproject1-hn05.4.7`
- type: task
- priority: P0
- labels: customs,l5,l5-phase-4,layer-5
- parent: `testproject1-hn05.4`
- blocked by: `testproject1-hn05.4.6`, `testproject1-hn05.4.5`, `testproject1-hn05.1.3`

#### Description

# l5-p4-measure-scanner-interactions

## SOURCE OF TRUTH WARNING

- **Spec**: §1.7.1, §1.7.2, §1.7.3, §1.7.4, §1.8.1, §1.8.2, §1.8.3, §1.8.4 (PRIMARY)
- **TDD**: `L5-T-measure_interaction-001..010` (PRIMARY), §7.3 (steel substitutive fixture), §11.5 (Error Vocabulary Contract)
- **Strategy**: Phase 4 (after UK10 resolution)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Owns `packages/intelligence/src/customs/measure-scanner.ts` (one of the 6 customs package files). Classifies every active HMRC measure on shortlisted UK10 branches into 5 interaction families (additive / substitutive / conditional / documentary_only / unknown). Preserves compound/specific duty structure (no flattening). Records supplementary unit requirements as first-class blockers. Bundle splitter GIR 3 set-candidate escape hatch.

## Files to Create/Modify

- **Create:** `packages/intelligence/src/customs/measure-scanner.ts`
- **Modify:** `packages/intelligence/src/customs/measure-interaction.test.ts` (add `L5-T-measure_interaction-001..010`)

## Spec Requirements (verbatim)

**L5-FR-1.7.1 — Measure interaction is family-specific, not simple stacking.**
- AC1: Additive families compute base duty plus AD/CVD or other additive measures.
- AC2: Substitutive families compute the specified substitution rule, such as greater-of safeguard or AD, without double-charging.
- AC3: Conditional families model quota exhaustion, additional-code eligibility, and rate switches.
- AC4: Unknown interaction rules emit `MEASURE_INTERACTION_UNKNOWN` and block auto-decision.

**L5-FR-1.7.2 — Additional-code satisfiability is modeled as a legal prerequisite.**
- AC1: HMRC DE 6/17 and other national additional-code requirements record prerequisites and documentary evidence.
- AC2: When satisfiability is unknown, the engine applies the rate that applies without the additional code.
- AC3: `additional_codes` and `additional_code_requirements_json` distinguish measure existence from truthful eligibility to claim the code.

**L5-FR-1.7.3 — Measure scanner classifies every active measure into an interaction family.**
- AC1: Each active HMRC measure on a shortlisted UK10 branch is labeled `additive`, `substitutive`, `conditional`, `documentary_only`, or `unknown` before duty computation.
- AC2: Unknown or unmapped interaction families emit `MEASURE_INTERACTION_UNKNOWN` and cannot auto-approve the affected branch.

**L5-FR-1.7.4 — Golden interaction cases cover additive, substitutive, and conditional families.**

**L5-FR-1.8.1 — Duty expressions preserve full component structure.**
- AC1: Ad valorem, specific per-unit, and compound measures are stored in `duty_breakdown_json`.
- AC2: Compound duties are never flattened to a single `dutyPct`.

**L5-FR-1.8.2 — Supplementary units are first-class blockers.**
- AC1: Required units such as kg net, litres, pairs, item count, or per-1000 quantity are recorded in `supplementary_units_required_json`.
- AC2: Missing required unit data emits `SUPPLEMENTARY_UNIT_MISSING`, abstains on that duty component, and does not average or estimate the missing unit.

**L5-FR-1.8.3 — Bundle splitter includes a GIR 3 set-candidate escape hatch.**
- AC1: Retail sets or kits that may classify by essential character or last-in-tariff-order are flagged for manual classification.
- AC2: Unsafe forced decomposition is prohibited.

**L5-FR-1.8.4 — Specific-duty unit conversion requires explicit compatible quantity evidence.**
- AC1: Specific or compound duties use only quantity units explicitly present in supplier quotes, extracted lot documents, packing scenarios, or operator-entered evidence.
- AC2: Incompatible conversions, such as pieces to kg without net-weight evidence, emit `SUPPLEMENTARY_UNIT_MISSING` rather than deriving an average weight.

## Implementation Details

```ts
export type InteractionFamily = "additive" | "substitutive" | "conditional" | "documentary_only" | "unknown";

export interface ActiveMeasure {
  measureType: string;          // e.g., "AD", "CVD", "safeguard", "MFN", "TRQ", "VAT"
  dutyExpression: DutyExpression;
  conditions?: { quotaCode?: string; additionalCode?: string };
}

export type DutyExpression =
  | { kind: "ad_valorem"; rateBps: number }
  | { kind: "specific"; unit: "kg" | "litre" | "pair" | "item" | "per_1000"; amountPence: number }
  | { kind: "compound"; components: DutyExpression[] }
  | { kind: "unknown" };

export interface MeasureScanResult {
  perMeasureClassification: Array<{ measure: ActiveMeasure; family: InteractionFamily }>;
  selectedDutyExpression: DutyExpression;        // after interaction resolution
  selectedDutyPence: number | null;              // null on UNKNOWN or SUPPLEMENTARY_UNIT_MISSING
  warnings: string[];
  blockers: string[];                             // contains MEASURE_INTERACTION_UNKNOWN, SUPPLEMENTARY_UNIT_MISSING
  dutyBreakdownJson: unknown;                     // structured payload for landed_cost_estimates.duty_breakdown_json
  measureInteractionJson: unknown;                // landed_cost_estimates.measure_interaction_json
  supplementaryUnitsRequiredJson: string[];       // list of required units (kg_net, litres, etc.)
  additionalCodesRequirementsJson: Array<{ code: string; satisfiability: "satisfied" | "unsatisfied" | "unknown"; evidenceRequired: string[] }>;
}

export function scanMeasures(opts: {
  uk10: string;
  measures: ActiveMeasure[];
  customsValuePence: number;
  supplementaryQuantities: Partial<Record<"kg_net" | "litre" | "pair" | "item" | "per_1000", number>>;
  exporterResolution: ExporterResolutionResult;
}): MeasureScanResult;
```

Interaction family rules:

- **Additive** (default for non-conflicting measures): base MFN + AD/CVD components sum.
- **Substitutive (greater-of safeguard or AD)**: pick max, don't sum. Fixture (`L5-T-measure_interaction-002`): customs value GBP 10,000, safeguard 25.0% (250,000 pence), AD 21.1% (211,000 pence) → substitutive selects 250,000 pence, NOT 461,000 pence.
- **Conditional** (quota / additional-code): switch based on documented eligibility.
- **Documentary_only**: documentary requirement, no rate impact.
- **Unknown**: emit `MEASURE_INTERACTION_UNKNOWN`, no auto-approval.

Compound duty: preserve `kind: "compound"` with full component array. Never collapse to `rateBps` only.

Supplementary unit: if measure requires `kg_net` and `supplementaryQuantities.kg_net === undefined`, emit `SUPPLEMENTARY_UNIT_MISSING` and set `selectedDutyPence = null` for that component (abstain). Do NOT compute average weight from pieces.

GIR 3 set: if `bundleSplitter` upstream flagged `gir_3_candidate`, this scanner emits `gir_3_manual_classification_required` blocker; no auto-classification.

Tests to add (TDD §8.3):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-measure_interaction-001` | T1 | additive steel fixture: components remain separate in `duty_breakdown_json` |
| `L5-T-measure_interaction-002` | T1 | substitutive steel: safeguard 25.0% / AD 21.1% on GBP 10,000 → 250,000 pence, NOT 461,000 |
| `L5-T-measure_interaction-003` | T1 | conditional quota: switches to out-of-quota when allocation proof absent |
| `L5-T-measure_interaction-004` | T1 | unknown family emits `MEASURE_INTERACTION_UNKNOWN`, blocks auto-decision |
| `L5-T-measure_interaction-005` | T1 | HMRC DE 6/17 additional-code: prerequisites separate from eligibility; unknown satisfiability → rate without additional code |
| `L5-T-measure_interaction-006` | T1 | every active measure labeled with interaction family before duty computation |
| `L5-T-measure_interaction-007` | T1 | compound duty preserved as structured components, never flattened to `dutyPct` |
| `L5-T-measure_interaction-008` | T1 | required kg/litre/pair/item/per-1000 missing → `SUPPLEMENTARY_UNIT_MISSING`, abstain |
| `L5-T-measure_interaction-009` | T1 | GIR 3 set flagged for manual; no unsafe decomposition |
| `L5-T-measure_interaction-010` | T1 | pieces-to-kg conversion without net-weight evidence → `SUPPLEMENTARY_UNIT_MISSING` |

## Tests to Pass

All 10 listed test IDs (`L5-T-measure_interaction-001..010`).

## Acceptance Criteria

- [ ] `measure-scanner.ts` exists.
- [ ] All 5 interaction families implementable.
- [ ] Substitutive steel fixture arithmetic exact: 250,000 pence selected, not 461,000.
- [ ] Compound duty structure preserved in `dutyBreakdownJson`.
- [ ] Supplementary unit abstention: no fabricated averages or conversions.
- [ ] GIR 3 set flag preserved through to operator surface.
- [ ] All 10 listed test IDs pass.

## Dependencies

- **Blocked by:** `l5-p4-uk10-resolution-branching`, `l5-p4-exporter-origin-resolution`, `l5-p1-error-exhaustive-consumers`.
- **Blocks:** `l5-p4-vat-additional-code-valuation`, `l5-p4-conservative-defaults-evidence-leverage`, `l5-p4-divergence-readiness-producers`.
- **Phase:** 4.

---

### l5-p4-vat-additional-code-valuation

- id: `testproject1-hn05.4.8`
- type: task
- priority: P0
- labels: customs,l5,l5-phase-4,layer-5
- parent: `testproject1-hn05.4`
- blocked by: `testproject1-hn05.4.7`, `testproject1-hn05.2.6`

#### Description

# l5-p4-vat-additional-code-valuation

## SOURCE OF TRUTH WARNING

- **Spec**: §1.9.1, §1.9.2, §1.10.1, §1.10.2 (PRIMARY)
- **TDD**: `L5-T-customs_classification-016`, `L5-T-measure_interaction-014,015,016,017` (PRIMARY)
- **Strategy**: Phase 4 (after measure scanner; FX-gated lanes follow Phase 2)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

VAT rate determination (standard / reduced / zero / unresolved) and VAT recovery (bps fraction, not binary) are separate decisions. Valuation mode distinguishes `estimation` (`1_proxy`) from `declaration` (`1_actual`). Material proxy/declaration divergence emits `VALUATION_MODE_CONFLICT`. FX-gated lane requires Phase 2 FX engine — must follow Phase 2 gate.

## Files to Create/Modify

- **Extend:** `packages/intelligence/src/customs/measure-scanner.ts` (VAT scanner extension within the measure scanner OR a separate `vat-resolution.ts` file — both acceptable provided package surface count stays at 6)
- **Extend:** `packages/intelligence/src/customs/measure-interaction.test.ts` (add `L5-T-measure_interaction-014,015,016,017`)
- **Modify:** `packages/intelligence/src/customs/customs-classification.test.ts` (add `L5-T-customs_classification-016`)

## Spec Requirements (verbatim)

**L5-FR-1.9.1 — VAT rate and VAT recovery are separate decisions.**
- AC1: `vatRateState` is one of `standard`, `reduced`, `zero`, or `unresolved`.
- AC2: `vatRecoveryRateBps` records 0-10000 bps recovery fraction and is not a binary flag.
- AC3: Unresolved VAT status defaults to standard rate.

**L5-FR-1.9.2 — VAT additional-code relief paths require evidence.**
- AC1: VATZ/VATR and family 6/17 relief paths are recorded only when commodity and documentary prerequisites are satisfied.
- AC2: Missing relief evidence preserves standard-rate treatment and records evidence leverage.

**L5-FR-1.10.1 — Valuation mode distinguishes estimation from declaration.**
- AC1: `estimation` uses marketplace-derived proxy values and stores `customs_valuation_method = 1_proxy`.
- AC2: `declaration` uses invoice/contract transaction values and stores `customs_valuation_method = 1_actual` unless Method 2-6 or `unknown` applies.

**L5-FR-1.10.2 — Invoice price is not automatically customs value.**
- AC1: Selling commissions, assists, royalties, and resale-proceeds shares are represented in valuation evidence or flagged unknown.
- AC2: Material divergence between proxy and declaration mode emits `VALUATION_MODE_CONFLICT`.

## Implementation Details

VAT resolution types:

```ts
export type VatRateState = "standard" | "reduced" | "zero" | "unresolved";

export interface VatResolution {
  vatRateState: VatRateState;
  vatRecoveryRateBps: number;     // 0-10000
  importVatTreatment: "standard_rate_default" | "reduced_rate_applied" | "zero_rate_applied" | "vatz_relief" | "vatr_relief" | "family_6_17_relief";
  evidenceLeverage: Array<{ leverageItem: string; marginDeltaBps: number; evidenceRequired: string[] }>;
}

export function resolveVat(opts: {
  uk10: string;
  measures: ActiveMeasure[];
  reliefClaims: Array<{ kind: "VATZ" | "VATR" | "family_6_17"; evidenceArtifactPaths: string[] }>;
}): VatResolution;
```

Valuation mode types:

```ts
export type ValuationMode = "estimation" | "declaration";
export type CustomsValuationMethod = "1_proxy" | "1_actual" | "2" | "3" | "4" | "5" | "6" | "unknown";

export interface ValuationOutput {
  valuationMode: ValuationMode;
  customsValuationMethod: CustomsValuationMethod;
  customsValuePence: number;
  conflictDetected: boolean;
  errorCode?: "VALUATION_MODE_CONFLICT";
  commissionsAssistsRoyaltiesStatus: "represented" | "unknown" | "absent";
}

export function resolveValuation(opts: {
  proxyPricePence: number | null;       // marketplace-derived
  declarationPricePence: number | null;  // invoice/contract
  commissionsPence?: number;
  assistsPence?: number;
  royaltiesPence?: number;
  resaleProceedsSharePence?: number;
  divergenceThresholdBps: number;        // configurable; named test constant
}): ValuationOutput;
```

VAT defaults (§1.12.1 conservative defaults):

- Unresolved VAT status → `standard` rate; `importVatTreatment = "standard_rate_default"`; `vatRecoveryRateBps = 0`.
- VATZ / VATR / family 6/17 relief: requires commodity + documentary prerequisites. Missing relief evidence → preserve standard rate, record `evidenceLeverage` with margin delta.

Valuation conflict: proxy and declaration prices both present and diverge by > `divergenceThresholdBps` → emit `VALUATION_MODE_CONFLICT`.

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-customs_classification-016` | T1 | unresolved VAT: `vat_rate_state = "unresolved"`, `import_vat_treatment = "standard_rate_default"`, `vat_recovery_rate_bps = 0`, evidence leverage recorded |
| `L5-T-measure_interaction-014` | T1 | VATZ/VATR/family 6/17 relief require commodity + documentary prerequisites; missing relief evidence preserves standard-rate |
| `L5-T-measure_interaction-015` | T1 | `vatRateState` + `vatRecoveryRateBps` separate; unresolved → standard; recovery remains fractional bps |
| `L5-T-measure_interaction-016` | T1 | estimation → `1_proxy`; declaration → `1_actual` unless Method 2-6 or `unknown` |
| `L5-T-measure_interaction-017` | T1 | proxy/declaration divergence with commissions/assists/royalties unknown → `VALUATION_MODE_CONFLICT` |

## Tests to Pass

- **`L5-T-customs_classification-016`** (T1)
- **`L5-T-measure_interaction-014`** (T1)
- **`L5-T-measure_interaction-015`** (T1)
- **`L5-T-measure_interaction-016`** (T1)
- **`L5-T-measure_interaction-017`** (T1)

## Acceptance Criteria

- [ ] VAT resolution exposes 4-state `vatRateState`.
- [ ] `vatRecoveryRateBps` is bps (not binary).
- [ ] Unresolved VAT defaults to standard rate.
- [ ] Valuation mode distinguishes proxy/declaration with explicit `customs_valuation_method` enum.
- [ ] `VALUATION_MODE_CONFLICT` emitted on material divergence (named threshold constant).
- [ ] All 5 listed test IDs pass.
- [ ] FX rate dependency: if VAT computation uses FX (rare, but possible for non-GBP commodity declarations), the engine consumes the FX engine output (Phase 2 dependency).

## Dependencies

- **Blocked by:** `l5-p4-measure-scanner-interactions`, `l5-p2-fx-cron-persistence-gate` (Phase 2 gate for FX-dependent valuation).
- **Blocks:** `l5-p4-conservative-defaults-evidence-leverage`, `l5-p4-divergence-readiness-producers`.
- **Phase:** 4.

---

### l5-p4-classification-freshness

- id: `testproject1-hn05.4.9`
- type: task
- priority: P0
- labels: customs,l5,l5-phase-4,layer-5
- parent: `testproject1-hn05.4`
- blocked by: `testproject1-hn05.4.4`, `testproject1-hn05.1.5`

#### Description

# l5-p4-classification-freshness

## SOURCE OF TRUTH WARNING

- **Spec**: §1.11.1, §1.11.2, §1.11.3, §1.11.4, §3.5.8, §3.7.5 (PRIMARY)
- **TDD**: `L5-T-customs_classification-022,023,024` (PRIMARY)
- **Strategy**: Phase 4 (freshness skeleton; cron roots from Phase 1)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Owns `packages/intelligence/src/customs/classification-freshness.ts` (one of the 6 customs package files). Implements 4 INDEPENDENT freshness vectors: structure, measures, remedies, FX. Freshness watchers enqueue recalculation/reclassification when their vector goes stale. Expired classification freshness downgrades bid readiness. Missed watcher run → stale marking + catch-up sweep.

## Files to Create/Modify

- **Create:** `packages/intelligence/src/customs/classification-freshness.ts`
- **Modify:** `packages/intelligence/src/customs/customs-classification.test.ts` (add `L5-T-customs_classification-022,023,024`)
- **Possibly extend:** `crates/daemon/tests/customs_classification.rs` for the cron-side persistence assertions (the daemon side of `L5-T-customs_classification-023`)

## Spec Requirements (verbatim)

**L5-FR-1.11.1 — L5 tracks four independent freshness vectors.**
- AC1: Structure, measures, remedies, and FX freshness are tracked separately through `tariffCheckedAt`, `measuresFreshUntil`, `tradeRemedyWatchState`, and `fxRateCheckedAt`.
- AC2: Expiry in one vector cannot be masked by freshness in another vector.

**L5-FR-1.11.2 — Freshness watchers enqueue recalculation or reclassification.**
- AC1: TRA investigation openings set `tradeRemedyWatchState = investigation_open` and enqueue affected commodity families for review.
- AC2: HMRC tariff structure or measure changes emit a tariff-change control event and enqueue affected cached classifications.
- AC3: FX monthly rollover enqueues affected landed-cost estimates for recalculation.
- AC4: Four new `CronJobDefinition` entries (covered by Phase 1 `l5-p1-cron-definitions`).

**L5-FR-1.11.3 — Expired classification freshness downgrades bid readiness.**
- AC1: If `classification_fresh_until` expires before the expected import window, the estimate records a blocker and cannot satisfy the L5→L6 readiness gate.
- AC2: Expired structure, measure, or remedy vectors cannot be cleared by fresh FX alone.

**L5-FR-1.11.4 — Freshness watcher failures are observable and conservative.**
- AC1: A missed watcher run beyond its freshness window marks affected commodity families stale and enqueues catch-up work.
- AC2: A multi-day daemon outage creates stale classification blockers on affected cached estimates until watcher catch-up completes.

**L5-FR-3.5.8 — Error code `TRADE_REMEDY_WATCH_STALE`.**
- AC1: Trigger is TRA investigation or measure change affecting cached classification; action requires reclassification.

**L5-FR-3.7.5 — Risk: tariff cache freshness gap after daemon downtime.**

## Implementation Details

```ts
export type TradeRemedyWatchState = "stable" | "investigation_open" | "provisional_measure_active" | "definitive_measure_changed" | "stale";

export interface FreshnessVectors {
  structureCheckedAt: string;          // ISO 8601
  measuresFreshUntil: string;
  tradeRemedyWatchState: TradeRemedyWatchState;
  fxRateCheckedAt: string;
}

export interface FreshnessSweepInput {
  uk10: string;
  vectors: FreshnessVectors;
  expectedImportWindow: { start: string; end: string };
  now: Date;
  configuredWindow: { structureDays: number; measuresDays: number; remedyDays: number; fxDays: number };
}

export interface FreshnessSweepResult {
  passing: boolean;
  expiredVectors: Array<"structure" | "measures" | "remedies" | "fx">;
  blockers: string[];          // e.g., TRADE_REMEDY_WATCH_STALE
  enqueueReclassification: boolean;
  enqueueRecalculation: boolean;
}

export function sweepFreshness(input: FreshnessSweepInput): FreshnessSweepResult;
```

Each vector is INDEPENDENT (§1.11.1 AC2): if `measures` is stale but `structure` is fresh, the estimate is blocked on measures regardless of structure freshness.

Cron handlers (already registered by `l5-p1-cron-definitions`):

| Cron | Vector | Action on Stale |
|------|--------|-----------------|
| `HMRC_TARIFF_NEWS_WEEKLY_CRON` | structure | emit `TARIFF_STRUCTURE_CHANGED`, enqueue affected cached classifications |
| `TRA_CASE_REGISTER_DAILY_CRON` | remedies | emit `TRA_INVESTIGATION_OPEN`, set `tradeRemedyWatchState`, enqueue affected commodity families |
| `HMRC_FX_MONTHLY_ROLLOVER_CRON` | fx | enqueue affected landed-cost estimates (Phase 2 owns the recalc itself) |
| `MARKETPLACE_SOURCE_HEALTH_CANARY_CRON` | n/a (source-health, not freshness) | no freshness action |

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-customs_classification-022` | T1 | structure/measures/remedies/FX tracked independently; stale measure does not invalidate fresh structure; fresh FX cannot clear stale measure/remedy |
| `L5-T-customs_classification-023` | T2 | tariff structure change emits `TARIFF_STRUCTURE_CHANGED` with affected UK10 codes; TRA case opening emits `TRA_INVESTIGATION_OPEN` and sets `tradeRemedyWatchState = investigation_open` |
| `L5-T-customs_classification-024` | T2 | missed watcher run beyond freshness window marks families stale, enqueues catch-up, blocks L5→L6 readiness until catch-up completes |

## Tests to Pass

- **`L5-T-customs_classification-022`** (T1)
- **`L5-T-customs_classification-023`** (T2)
- **`L5-T-customs_classification-024`** (T2)

## Acceptance Criteria

- [ ] `classification-freshness.ts` exists.
- [ ] 4 vectors independent — assertion that fresh-on-one cannot clear stale-on-another.
- [ ] `TRADE_REMEDY_WATCH_STALE` emitted via daemon enum.
- [ ] Daemon cron handlers wired to the right vector via `l5-p1-cron-definitions`.
- [ ] Missed watcher beyond freshness window → catch-up enqueued.
- [ ] All 3 listed test IDs pass.

## Dependencies

- **Blocked by:** `l5-p1-cron-definitions`, `l5-p4-tariff-graph-junction-writes`.
- **Blocks:** `l5-p4-divergence-readiness-producers`, `l5-p4-customs-scope-preservation-gate`.
- **Phase:** 4.

---

### l5-p4-conservative-defaults-evidence-leverage

- id: `testproject1-hn05.4.10`
- type: task
- priority: P0
- labels: customs,l5,l5-phase-4,layer-5
- parent: `testproject1-hn05.4`
- blocked by: `testproject1-hn05.4.8`, `testproject1-hn05.4.7`, `testproject1-hn05.4.6`, `testproject1-hn05.2.6`

#### Description

# l5-p4-conservative-defaults-evidence-leverage

## SOURCE OF TRUTH WARNING

- **Spec**: §1.12.1, §1.12.2, §1.12.3, §1.12.4, §3.3.12 (PRIMARY)
- **TDD**: `L5-T-customs_classification-013,015,017,019` (PRIMARY)
- **Strategy**: Phase 4 (after exporter + measure scanner; FX dep through Phase 2)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Conservative defaults are mandatory and cannot be relaxed to improve apparent margin. This bead binds the defaults to a typed evidence-leverage system: every active default has a deterministic trigger, fixture, and computed `unlockable_margin_bps`. `confidence_breakdown.evidence_leverage[]` entries are decision-grade artifacts, not prose hints.

## Files to Create/Modify

- **Create:** `packages/intelligence/src/customs/conservative-defaults.ts` (NEW file — but note this is INSIDE the customs package and is the 6th customs file IF we don't already have it. The 6 customs files per §1.16: description-quality, multi-source-recall, ten-digit-resolution, measure-scanner, exporter-resolution, classification-freshness. Conservative-defaults is a logical sub-module — either fold into measure-scanner OR add as a non-counted helper file. RECOMMENDED: fold into one of the 6 named files via export. The package inventory test `L5-T-freight_engine-017` counts 6 customs files; we must stay at 6.)
- **Modify:** `packages/intelligence/src/customs/measure-scanner.ts` OR `exporter-resolution.ts` to host the conservative-defaults logic without adding a new file.
- **Modify:** `packages/intelligence/src/customs/customs-classification.test.ts` (add `L5-T-customs_classification-013,015,017,019`)

## Spec Requirements (verbatim)

**L5-FR-1.12.1 — Conservative defaults apply whenever resolution is incomplete.**
- AC1: The defaults in the table below are mandatory and cannot be relaxed to improve apparent margin.
- AC2: Each active default is recorded in `confidence_breakdown`.

| Parameter | Default | Rationale |
|-----------|---------|-----------|
| Origin preference | MFN rate, not preferential | No origin proof available |
| Exporter rate | Residual remedy rate, not named-exporter | No exporter proof |
| Quota allocation | Out-of-quota rate | No quota allocation proof |
| VAT rate | Standard rate | Unresolved VAT status |
| Incoterm | Boundary scenarios | No stated Incoterm |
| Supplementary units | Abstain on duty for that component | Unresolved unit data |

**L5-FR-1.12.2 — Conservative defaults surface unlockable margin instead of relaxing assumptions.**
- AC1: `confidence_breakdown.unlockable_margin_bps` stores margin improvement available if documented evidence replaces resolvable conservative defaults.
- AC2: `confidence_breakdown.evidence_leverage` stores per-parameter evidence needed and margin delta.
- AC3: Evidence leverage is computed for named exporter proof, origin proof, quota allocation, VAT relief, Incoterm clarification, and supplementary units when applicable.

**L5-FR-1.12.3 — Each conservative default has a deterministic trigger and fixture.**
- AC1: The 6 default rows above are triggered by absence of origin proof, exporter proof, quota allocation proof, VAT status, Incoterm, or compatible supplementary-unit data respectively.
- AC2: `customs-classification.test.ts` or `measure-interaction.test.ts` includes at least one fixture for each default row.

**L5-FR-1.12.4 — Evidence leverage margin deltas are differential recalculations.**
- AC1: For each leverage item, `margin_delta_bps` is the difference between `net_margin_base_bps` under the evidence-resolved branch and `net_margin_base_bps` under the conservative default, using the same quote, freight, and FX snapshot.
- AC2: Step 03 TDD must lock any category-specific arithmetic details before bead creation, and L5 implementation must emit decision-grade `confidence_breakdown.evidence_leverage[]` entries rather than prose-only hints.
- AC3: If evidence cannot change the conservative outcome, `margin_delta_bps = 0` and the leverage item is omitted unless the missing evidence remains a hard blocker.

**L5-FR-3.3.12 — Scenario: conservative-to-actual uplift measurement.**

## Implementation Details

Evidence-leverage type:

```ts
export interface EvidenceLeverageItem {
  parameter: "origin_preference" | "exporter_rate" | "quota_allocation" | "vat_rate" | "incoterm" | "supplementary_units";
  defaultApplied: string;             // e.g., "MFN", "residual_default", "out_of_quota", "standard_rate_default", "boundary_scenarios", "abstained"
  evidenceRequired: string[];         // e.g., ["origin_certificate", "supplier_profile_naming_manufacturer"]
  marginDeltaBps: number;             // positive bps if evidence improves outcome
  unlockable: boolean;                // true if evidence could change outcome
}

export interface ConfidenceBreakdown {
  unlockable_margin_bps: number;       // sum of margin_delta_bps where unlockable
  evidence_leverage: EvidenceLeverageItem[];
  active_defaults: string[];           // human-readable list per §1.12.1 AC2
}

export function computeConfidenceBreakdown(opts: {
  exporterResolution: ExporterResolutionResult;
  vatResolution: VatResolution;
  freightOutput: FreightEngineOutputs;
  measureScanResult: MeasureScanResult;
  customsValuePence: number;
  fxBasisDelta: FxBasisDelta;
  // ...other inputs as needed for differential margin
}): ConfidenceBreakdown;
```

Differential margin math (§1.12.4):

- For each leverage item, recompute `net_margin_base_bps` under the evidence-resolved branch using the SAME quote, freight, and FX snapshot.
- `margin_delta_bps = net_margin_resolved - net_margin_default`.
- If `margin_delta_bps = 0` AND the missing evidence is NOT a hard blocker → omit the item.
- If `margin_delta_bps > 0` OR missing evidence is a hard blocker → record the item.

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-customs_classification-013` | T1 | missing origin proof: `origin_evidence_state = "mfn_default_missing_origin_proof"`, MFN duty applied, `confidence_breakdown.origin_preference.default_applied = true`, positive `margin_delta_bps` only when origin proof would alter margin |
| `L5-T-customs_classification-015` | T1 | missing quota allocation: `additional_code_requirements_json[].satisfiability = "unknown"`, out-of-quota rate applied, quota allocation proof recorded as evidence leverage |
| `L5-T-customs_classification-017` | T1 | missing Incoterm: EXW/FOB/CIF boundary scenarios run; required logistics evidence stored; select highest feasible landed-cost branch rather than relaxing freight assumptions (overlap with `l5-p3-freight-boundary-defaults` — agree on fixture) |
| `L5-T-customs_classification-019` | T1 | each `confidence_breakdown.evidence_leverage[].margin_delta_bps` recomputed against same quote/freight/FX snapshot; unchanged outcomes omit zero-delta soft hints |

## Tests to Pass

- **`L5-T-customs_classification-013`** (T1)
- **`L5-T-customs_classification-015`** (T1)
- **`L5-T-customs_classification-017`** (T1)
- **`L5-T-customs_classification-019`** (T1)

## Acceptance Criteria

- [ ] 6 conservative defaults each have deterministic trigger.
- [ ] Each default emits a typed `EvidenceLeverageItem`.
- [ ] `margin_delta_bps` is a differential recalculation against the SAME snapshot.
- [ ] Zero-delta items omitted unless evidence is a hard blocker.
- [ ] Customs package surface stays at 6 files (conservative-defaults logic folded into measure-scanner OR exporter-resolution).
- [ ] All 4 listed test IDs pass.

## Dependencies

- **Blocked by:** `l5-p4-exporter-origin-resolution`, `l5-p4-measure-scanner-interactions`, `l5-p4-vat-additional-code-valuation`, `l5-p2-fx-cron-persistence-gate`.
- **Blocks:** `l5-p4-divergence-readiness-producers`, `l5-p4-customs-scope-preservation-gate`.
- **Phase:** 4.

---

### l5-p4-divergence-readiness-producers

- id: `testproject1-hn05.4.11`
- type: task
- priority: P0
- labels: customs,l5,l5-phase-4,l6-readiness,layer-5
- parent: `testproject1-hn05.4`
- blocked by: `testproject1-hn05.4.10`, `testproject1-hn05.4.9`, `testproject1-hn05.4.8`, `testproject1-hn05.4.7`, `testproject1-hn05.4.5`

#### Description

# l5-p4-divergence-readiness-producers

## SOURCE OF TRUTH WARNING

- **Spec**: §1.13.1, §1.13.2, §3.8.1, §3.8.2 (PRIMARY)
- **TDD**: `L5-T-customs_classification-020,021,034` (PRIMARY)
- **Strategy**: Phase 4 (final customs lane before phase gate)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Produces and persists the L5→L6 readiness blockers. The L6 bid workflow cannot reach `ready_to_bid` while any of: `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, required-component `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`, or expired `classification_fresh_until` is active. Floor-only quote-role gate also produces here (overlap with Phase 5 — Phase 5 owns quote-role tagging, Phase 4 owns the gate check). Material branch divergence is a hard blocker, never averaged.

## Files to Create/Modify

- **Modify:** `packages/intelligence/src/customs/ten-digit-resolution.ts` (divergence detection)
- **Modify:** `packages/intelligence/src/customs/measure-scanner.ts` (decision_grade computation)
- **Modify:** `packages/intelligence/src/customs/customs-classification.test.ts` (add `L5-T-customs_classification-020,021,034`)

## Spec Requirements (verbatim)

**L5-FR-1.13.1 — Material branch divergence is a hard blocker.**
- AC1: UK10 branches split on remedy/no-remedy, duty type, or material duty spread emit `CLASSIFICATION_AMBIGUOUS` or `TARIFF_BRANCH_DIVERGENCE`.
- AC2: Divergent branches are never averaged into one actionable estimate.
- AC3: Until gold-set calibration replaces it with a documented test constant, `customs_cost_spread_bps >= 1500` emits `TARIFF_BRANCH_DIVERGENCE`; remedy/no-remedy and duty-type splits block regardless of spread bps.

**L5-FR-1.13.2 — Screening-grade auto-pass is allowed only under all three safeguards.**
- AC1: `worstCaseNetMargin >= gate`.
- AC2: `customsCostSpread` is immaterial under the calibrated threshold and all branches remain in the same measure family.
- AC3: No catastrophic legal branch exists, including residual trade remedy, missing supplementary unit, unresolved additional-code satisfiability, or expired freshness for the expected import window.

**L5-FR-3.8.1 — L5 exposes the L6 bid-readiness gate.**
- AC1: L6 cannot reach `ready_to_bid` while `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, required-component `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`, or expired `classification_fresh_until` is active.
- AC2: Quote-role quality gate blocks opportunities backed only by `domestic_wholesale_floor` or `optimistic_floor` unless the operator explicitly overrides.
- AC3: Gate producers are explicit: `L5-FR-3.5.1`, `L5-FR-3.5.2`, `L5-FR-3.5.5`, `L5-FR-2.4.1`/`L5-FR-1.13.2` for `decision_grade = broker_required`, and `L5-FR-1.11.3` for expired `classification_fresh_until`.

**L5-FR-3.8.2 — L1 PDF extraction can later support customs evidence overlays.**

## Implementation Details

Customs spread threshold constant (TDD §7.3):

```ts
export const CUSTOMS_BRANCH_DIVERGENCE_BPS = 1500;
```

Divergence detector:

```ts
export interface DivergenceCheckResult {
  divergent: boolean;
  divergenceKind?: "remedy_no_remedy" | "duty_type" | "material_spread";
  customsCostSpreadBps: number;
  emittedCodes: Array<"CLASSIFICATION_AMBIGUOUS" | "TARIFF_BRANCH_DIVERGENCE">;
  selectedBranchAuto: TenDigitCandidate | null;        // null when divergent
  worstCaseDutyPence: number;
  bestCaseDutyPence: number;
}

export function detectBranchDivergence(branches: TenDigitCandidate[]): DivergenceCheckResult;
```

Decision-grade rollup:

```ts
export type DecisionGrade = "auto_pass" | "review_required" | "broker_required";

export interface DecisionGradeInput {
  divergence: DivergenceCheckResult;
  measureScan: MeasureScanResult;             // for MEASURE_INTERACTION_UNKNOWN, SUPPLEMENTARY_UNIT_MISSING
  exporterRateBasis: ExporterRateBasis;       // residual = caution
  freshnessSweep: FreshnessSweepResult;        // expired vectors = broker
  worstCaseNetMarginBps: number;
  gateBps: number;
  customsCostSpreadBps: number;
  netGateCoverage: { allBranchesSameFamily: boolean };
}

export function classifyDecisionGrade(input: DecisionGradeInput): { grade: DecisionGrade; blockers: string[]; warnings: string[] };
```

Decision-grade rules:

- **`broker_required`**: any of — `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, required-component `SUPPLEMENTARY_UNIT_MISSING`, `MEASURE_INTERACTION_UNKNOWN`, unresolved additional-code satisfiability where unsatisfied means broker, valuation conflict, GIR 3 manual, dangerous packaging subfamily, residual trade remedy (no proof) with active remedy, expired structure/measures/remedies/fx vector.
- **`review_required`**: residual with no remedy active; warnings without blockers; floor-only quote-role.
- **`auto_pass`**: all 3 safeguards from §1.13.2 met — worstCaseNetMargin ≥ gate, customsCostSpread < CUSTOMS_BRANCH_DIVERGENCE_BPS AND same measure family across branches, no catastrophic legal branch.

L6 readiness gate trace (from strategy "L5→L6 Readiness Gate Trace" table):

| Gate Condition | Producer in this bead | Test |
|----------------|----------------------|------|
| `CLASSIFICATION_AMBIGUOUS` | top HS6 disagreement OR top-3 spread material | `L5-T-customs_classification-020` |
| `TARIFF_BRANCH_DIVERGENCE` | UK10 splits on remedy/no-remedy/duty-type/material spread | `L5-T-customs_classification-020` |
| Required `SUPPLEMENTARY_UNIT_MISSING` | (Phase 4 measure scanner emits; this bead persists into `blockers`) | `L5-T-customs_classification-018` (cross-ref) |
| `decision_grade = broker_required` | classifyDecisionGrade() output | `L5-T-customs_classification-034` |
| Expired `classification_fresh_until` | freshness sweep (Phase 4 freshness bead) | `L5-T-customs_classification-022` |
| Floor-only quote-role | Phase 5 owns role tagging; Phase 4 gate persists block | `L5-T-customs_classification-034` |

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-customs_classification-020` | T1 | remedy/no-remedy split, duty-type split, or `customs_cost_spread_bps >= 1500` emits `TARIFF_BRANCH_DIVERGENCE` or `CLASSIFICATION_AMBIGUOUS`; never averages branches |
| `L5-T-customs_classification-021` | T1 | screening-grade auto-pass requires worst-case net margin ≥ gate, immaterial spread, same measure family, no catastrophic legal branch |
| `L5-T-customs_classification-034` | T1 | L6 fixture: L6 cannot reach `ready_to_bid` while L5 blockers, broker-required grade, expired freshness, or floor-only quote role active without explicit operator override |

## Tests to Pass

- **`L5-T-customs_classification-020`** (T1)
- **`L5-T-customs_classification-021`** (T1)
- **`L5-T-customs_classification-034`** (T1)

## Acceptance Criteria

- [ ] `CUSTOMS_BRANCH_DIVERGENCE_BPS = 1500` named constant exported.
- [ ] Divergent branches are NEVER averaged.
- [ ] 5 L6 readiness gate conditions all producible.
- [ ] `decisionGrade` enum returned; never null/unknown.
- [ ] Floor-only quote-role gate persists block in `landed_cost_estimates.blockers`.
- [ ] All 3 listed test IDs pass.

## Dependencies

- **Blocked by:** `l5-p4-uk10-resolution-branching`, `l5-p4-measure-scanner-interactions`, `l5-p4-vat-additional-code-valuation`, `l5-p4-conservative-defaults-evidence-leverage`, `l5-p4-classification-freshness`.
- **Blocks:** `l5-p4-customs-scope-preservation-gate`, `l5-p6-final-dod-readiness-gate`.
- **Phase:** 4.

---

### l5-p4-redaction-sentinel-design

- id: `testproject1-hn05.4.12`
- type: task
- priority: P0
- labels: l5,l5-phase-4,layer-5,redaction
- parent: `testproject1-hn05.4`
- blocked by: `testproject1-hn05.1.1`

#### Description

# l5-p4-redaction-sentinel-design

## SOURCE OF TRUTH WARNING

- **Spec**: §1.3.1, §3.8.1 (PRIMARY)
- **TDD**: `L5-T-measure_interaction-020`, §11.8 (Redaction Boundary Contract) (PRIMARY)
- **Strategy**: Phase 4 design; Phase 6 implementation of the field-level boundary

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Designs the redaction sentinel taxonomy. Phase 4 designs WHICH new L5 decision-grade fields are automation-sensitive vs control vs display. Phase 6 implements the actual projection boundary. This bead emits a design artifact (markdown or YAML doc) + a redaction sentinel fixture that `L5-T-measure_interaction-020` can read.

## Files to Create/Modify

- **Create:** `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-redaction-sentinel-design.md` (taxonomy doc)
- **Create:** `tests/fixtures/l5/redaction/sentinels.yaml` (fixture mapping field name → classification + sentinel value)
- **Modify:** `packages/intelligence/src/customs/measure-interaction.test.ts` (add `L5-T-measure_interaction-020` skeleton; full implementation in Phase 6)

## Spec Requirements (verbatim)

**L5-FR-1.3.1 — L5 populates existing `landed_cost_estimates` columns without adding columns to that table.**
- AC1: All decision-grade `landed_cost_estimates` fields listed in §2.4 are populated through existing nullable/non-null columns from `migrations/0001_sqlite_state_initial.sql`.
- AC2: L5 does not add new columns to `landed_cost_estimates`.

**L5-FR-3.8.1 — L5 exposes the L6 bid-readiness gate.**
- AC2: Quote-role quality gate blocks opportunities backed only by `domestic_wholesale_floor` or `optimistic_floor` unless the operator explicitly overrides.

## Implementation Details

§11.8 redaction boundary contract (verbatim):

| Field / Payload | Classification | Viewer Projection | Operator Projection |
|-----------------|----------------|-------------------|---------------------|
| `declarable_commodity_code_10`, `net_margin_low_bps`, `net_margin_base_bps`, `net_margin_high_bps`, `decision_grade`, `warnings`, `blockers`, `cbam_exposure_flag` | display | present | present |
| `customs_fx_rate`, `commercial_fx_rate`, `fx_basis_delta_bps`, `freight_low_gbp_pence`, `freight_base_gbp_pence`, `freight_high_gbp_pence`, `customs_duty_gbp_pence`, `trade_remedy_duty_gbp_pence` | control | present only as rounded/decision-grade values; raw source artifacts absent | present |
| `exporter_rate_basis`, `exporter_identity_confidence_bps`, `origin_evidence_state`, `trade_remedy_cases_json`, `duty_breakdown_json`, `measure_interaction_json`, `confidence_breakdown.evidence_leverage` | automation-sensitive | absent by field name and sentinel value | present |
| browser transcripts, raw marketplace pages, skill checkpoints, service keys, credential state | local-only sensitive | absent | absent from shared projection; operator can access only local desktop artifacts |

Sentinel taxonomy doc must:

1. List every new L5 field added to `landed_cost_estimates` and `product_commodity_codes`.
2. Classify each as display / control / automation-sensitive / local-only sensitive.
3. For automation-sensitive fields, document the unique sentinel value the test fixture will use (e.g., a UUID-style marker that the test seeds and asserts presence/absence by field name AND sentinel value).
4. Map to existing L4 RLS policy file (`infra/supabase/migrations/0004_rls_policies.sql`) and identify whether any new RLS rule or redacted view is needed (Phase 6 implements).

Sentinel fixture (`tests/fixtures/l5/redaction/sentinels.yaml`):

```yaml
display_fields:
  declarable_commodity_code_10:
    sentinel: "L5-SENTINEL-DISPLAY-4819100000-2026"
    viewer_visible: true
    operator_visible: true
  # ... (8 display fields)

control_fields:
  customs_fx_rate:
    sentinel: 0.78340001                     # numeric sentinel
    viewer_visible: true                      # rounded; raw artifact absent
    operator_visible: true
  # ... (8 control fields)

automation_sensitive_fields:
  exporter_rate_basis:
    sentinel: "L5-SENTINEL-AUTOSENSITIVE-RESIDUAL-DEFAULT-2026"
    viewer_visible: false                     # MUST be filtered by field name AND sentinel value
    operator_visible: true
  # ... (7 automation-sensitive fields)

local_only_sensitive:
  browser_transcripts:
    sentinel: "/tmp/transcripts/l5-redaction-sentinel-001.txt"
    viewer_visible: false
    operator_visible: "local_artifact_only"
```

Tests skeleton (full impl in Phase 6):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-measure_interaction-020` | T2 | viewer-role projection can read display + rounded control fields; cannot read automation-sensitive sentinels; operator role includes operator/control fields; both roles read-only outside approved RPCs |

## Tests to Pass

- **`L5-T-measure_interaction-020`** (T2) — skeleton only; full implementation closes in Phase 6 `l5-p6-field-level-projection-boundary`.

## Acceptance Criteria

- [ ] Sentinel taxonomy doc exists.
- [ ] Sentinel fixture YAML covers every new L5 field by classification.
- [ ] No new columns added to `landed_cost_estimates` (re-verified).
- [ ] Phase 6 has enough design to implement the field-level projection boundary.
- [ ] Test skeleton for `L5-T-measure_interaction-020` exists with seed/assertion stubs.

## Dependencies

- **Blocked by:** `l5-p1-sqlite-product-commodity-codes` (knows new fields), L4 RLS policy file present.
- **Blocks:** `l5-p4-customs-scope-preservation-gate`, `l5-p6-field-level-projection-boundary`.
- **Phase:** 4.

---

### l5-p4-customs-scope-preservation-gate

- id: `testproject1-hn05.4.13`
- type: task
- priority: P0
- labels: l5,l5-phase-4,layer-5,review-gate
- parent: `testproject1-hn05.4`
- blocked by: `testproject1-hn05.4.12`, `testproject1-hn05.4.11`, `testproject1-hn05.4.10`, `testproject1-hn05.4.9`, `testproject1-hn05.4.8`, `testproject1-hn05.4.7`, `testproject1-hn05.4.6`, `testproject1-hn05.4.5`, `testproject1-hn05.4.4`, `testproject1-hn05.4.3`, `testproject1-hn05.4.2`, `testproject1-hn05.4.1`

#### Description

# l5-p4-customs-scope-preservation-gate

## SOURCE OF TRUTH WARNING

- **Spec**: §1.4-1.13 (all customs sections) (PRIMARY)
- **TDD**: `L5-T-hmrc_fpo-*`, `L5-T-customs_classification-001..024,033,034,036`, `L5-T-measure_interaction-001..020` (PRIMARY)
- **Strategy**: Phase 4 gate

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Phase 4 review gate. Verifies the entire customs engine is integrated. The largest phase by bead count (13 task beads). Closes the L5→L6 readiness producer surface so Phase 6 has all blocker conditions available.

## Files to Inspect/Modify

- All Phase 4 task bead outputs.
- **Extend:** `crates/daemon/tests/customs_classification.rs` with the persistence tests citing `L5-T-customs_classification-004,007,022,023,024,036`, `L5-T-measure_interaction-019,020`.

## Gate Criteria (strategy §Phase 4)

1. All `hmrc-fpo.test.ts`, `customs-classification.test.ts`, `measure-interaction.test.ts` Phase 4 IDs pass.
2. Multi-family expansion exercises at least 3 plausible HS6 families before UK10 shortlisting.
3. `product_commodity_codes` writes use candidate-level set replacement under one `stage_run_id`.
4. Conservative defaults + all L5 customs blockers emit through typed error paths.
5. L5→L6 readiness producers exist for classification ambiguity, tariff branch divergence, required supplementary-unit missing, broker-required decision grade, expired classification freshness.
6. Redaction/sync sentinel design ready for Phase 6.

## Implementation Details

Phase 4 closes a large surface. This bead also covers the §3.3.10 category whitelist scenario:

| Test ID | Where it lives | Owner |
|---------|----------------|-------|
| `L5-T-customs_classification-007` | landed_cost_estimates field activation (no DDL change) | needs cross-bead persistence test |
| `L5-T-customs_classification-010` | category whitelist (production gating) | overlap with Phase 6 (`l5-p6-final-dod-readiness-gate` owns full DoD walkthrough) |
| `L5-T-customs_classification-012` | full pipeline fixture — multi-source recall, UK10 resolution, exporter resolution, measure interaction, additional-code satisfiability, VAT, freshness, and conservative defaults all run before net-margin gating (covers `L5-FR-1.2.3` + `L5-FR-1.1.1`) | this bead — integrated customs+FX+freight output runs end-to-end |
| `L5-T-customs_classification-033` | open-question runtime behavior | this bead OR `l5-p4-classification-freshness` |
| `L5-T-customs_classification-036` | complete-row guard (`landed_cost_estimates`) | this bead — runs against the integrated customs+FX+freight output |

Verification checklist:

```bash
bun test packages/intelligence/src/customs/hmrc-fpo.test.ts
bun test packages/intelligence/src/customs/customs-classification.test.ts
bun test packages/intelligence/src/customs/measure-interaction.test.ts

# 12 hmrc_fpo IDs
for i in 001 002 003 004 005 006 007 008 009 010 011 012; do
  bun test packages/intelligence/src/customs/hmrc-fpo.test.ts 2>&1 | rg -q "L5-T-hmrc_fpo-$i" || exit 1
done

# 36 customs_classification IDs (Phase 4 owns 001..024, 033, 034, 036)
for i in 001 002 003 004 005 006 007 008 009 010 011 012 013 014 015 016 017 018 019 020 021 022 023 024 033 034 036; do
  bun test packages/intelligence/src/customs/customs-classification.test.ts 2>&1 | rg -q "L5-T-customs_classification-$i" || exit 1
done

# 20 measure_interaction IDs (Phase 4 owns 001..020)
for i in 001 002 003 004 005 006 007 008 009 010 011 012 013 014 015 016 017 018 019 020; do
  bun test packages/intelligence/src/customs/measure-interaction.test.ts 2>&1 | rg -q "L5-T-measure_interaction-$i" || exit 1
done

cargo test --test customs_classification
```

L6 readiness producer trace (verify each blocker can be produced from persisted L5 evidence):

```bash
# CLASSIFICATION_AMBIGUOUS
# TARIFF_BRANCH_DIVERGENCE
# SUPPLEMENTARY_UNIT_MISSING (required component)
# decision_grade = broker_required
# Expired classification_fresh_until
```

This bead is also where `L5-T-customs_classification-007` (full `landed_cost_estimates` row population without DDL change) is asserted on the integrated output. `L5-T-customs_classification-008` (Alibaba/1688/Amazon/eBay rows in `supplier_quotes`) closes in Phase 5 once supplier quotes are emitted, but this bead asserts the schema activation is in place.

## Tests to Pass

- All Phase 4 listed test IDs: `L5-T-hmrc_fpo-001..012`, `L5-T-customs_classification-001..024,033,034,036`, `L5-T-measure_interaction-001..020`.

## Acceptance Criteria

- [ ] All Phase 4 test IDs pass.
- [ ] `customs_classification.rs` daemon integration target passes.
- [ ] L5→L6 readiness producers are all wireable from persisted evidence.
- [ ] Roll-back conditions from strategy §Phase 4 NOT triggered (no greedy descent; TARIC doesn't overwrite HMRC; no averaged branches; readiness blockers persist).
- [ ] `landed_cost_estimates` has no new columns (schema diff guard).

## Dependencies

- **Blocked by:** all 12 prior Phase 4 task beads.
- **Blocks:** `l5-p6-accuracy-harness-module`, `l5-p6-comparison-adapters`, `l5-p6-mainrs-feature-flag-shadow`, `l5-p6-final-dod-readiness-gate`.
- **Phase:** 4 (final).

---

## Phase 5 Task Beads

### l5-p5-validator-overlay-contract

- id: `testproject1-hn05.5.1`
- type: task
- priority: P0
- labels: l5,l5-phase-5,layer-5,validator
- parent: `testproject1-hn05.5`
- blocked by: `testproject1-hn05.1.8`

#### Description

# l5-p5-validator-overlay-contract

## SOURCE OF TRUTH WARNING

- **Spec**: §2.3.4 AC3, §2.3.5 AC2, §3.7.7 (PRIMARY)
- **TDD**: `L5-T-freight_engine-013,018` (PRIMARY)
- **Strategy**: Phase 5 (decided early so all promoted sources bind compatible versions)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

The validator overlay contract binds worker behavior versions to validator overlay versions for every promoted L5 marketplace source. This bead creates the customs-classification validator overlay AND establishes the contract for compatible version binding. Strategy §Phase 5 ordering: "Customs validator overlay is decided early in this phase so all promoted marketplace skills bind compatible worker/validator behavior versions."

## Files to Create/Modify

- **Create:** `skills/validation/overlay-customs-classification/SKILL.md`
- **Create:** `skills/validation/overlay-customs-classification/validator-rubric.yaml`
- **Create:** `skills/validation/overlay-customs-classification/result.schema.json`
- **Create:** `skills/validation/overlay-customs-classification/checkpoint.schema.json`
- **Create:** `skills/validation/overlay-customs-classification/canary.inputs.jsonl`
- **Create:** `skills/validation/overlay-customs-classification/golden.assertions.yaml`

## Spec Requirements (verbatim)

**L5-FR-2.3.4 — Marketplace source registration is explicit.**
- AC3: Validator overlay binding records worker/validator behavior versions per source.

**L5-FR-2.3.5 — Skill promotion uses the full production directory and validator standard.**
- AC1: eBay, Alibaba, 1688, Amazon, and PCS production bindings include `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, and `validator-rubric.yaml`.
- AC2: Stable promotion records compatible `worker_behavior_version` and `validator_behavior_version`; incompatible versions block promotion rather than silently recalibrating.

**L5-FR-3.7.7 — Risk: worker and validator overlay version drift.**
- AC1: Mitigation is explicit worker/validator behavior version binding and stable-promotion blocking on incompatible versions.

## Implementation Details

`skills/validation/overlay-customs-classification/SKILL.md` frontmatter:

```yaml
---
skill_id: validation/overlay-customs-classification
version: 1.0.0
promotion_stage: candidate
interaction_profile: contract
model_profile: tier_2_audit  # mid-cost reasoning model
auth_requirements: none
overlay_target_skills:
  - landed-cost/hmrc-customs-classification
  - landed-cost/taric-cross-check
  - customs/multi-source-recall
  - customs/measure-scanner
worker_behavior_version_compat:
  - "1.x"  # initial compatibility range
validator_behavior_version: "1.0.0"
---
```

`validator-rubric.yaml`:

```yaml
overlay_id: validation/overlay-customs-classification
target_field_checks:
  - field: candidate_recall_agreement
    rule: "must have exactly 5 source-family entries"
    severity: blocker
  - field: declarable_commodity_code_10
    rule: "must be UK10 (10-digit) when non-null"
    severity: blocker
  - field: exporter_rate_basis
    rule: "must be in {named_exporter, annex_exporter, residual_default}"
    severity: blocker
  - field: duty_breakdown_json
    rule: "must preserve compound duty components when applicable"
    severity: blocker
  - field: customs_cost_spread_bps
    rule: "must trigger TARIFF_BRANCH_DIVERGENCE when >= 1500 OR remedy/no-remedy split"
    severity: blocker
  - field: decision_grade
    rule: "must be in {auto_pass, review_required, broker_required}"
    severity: blocker
agent_claimed_vs_daemon_observed:
  - "agent_claimed_multi_family_expansion vs daemon_observed_query_count"
  - "agent_claimed_taric_consulted vs daemon_observed_taric_skill_invocation_log"
basis_sufficiency:
  - "Validator can decline with INSUFFICIENT_BASIS if recall produced fewer than 2 independent source families"
calibration_corpora:
  - tests/fixtures/calibration/customs-classification/fasteners/
  - tests/fixtures/calibration/customs-classification/packaging-cartons/
  - tests/fixtures/calibration/customs-classification/examination-gloves/
```

`result.schema.json`: standard validation result envelope (verdict ∈ approved/rejected/flagged/insufficient_basis, structured_reasons array, basis_sufficiency_state).

Canary inputs (5 minimum per bootstrap §7.2 marketplace convention; this overlay is not strictly marketplace, but the JSONL+YAML pair pattern still applies):

```jsonl
{"case_id":"l5-overlay-fasteners-clean-001","worker_output_path":"tests/fixtures/calibration/customs-classification/fasteners/clean/case-001.json","expected_verdict":"approved"}
{"case_id":"l5-overlay-fasteners-mutated-001","worker_output_path":"tests/fixtures/calibration/customs-classification/fasteners/mutated/case-001.json","expected_verdict":"rejected"}
{"case_id":"l5-overlay-cartons-clean-001","worker_output_path":"tests/fixtures/calibration/customs-classification/packaging-cartons/clean/case-001.json","expected_verdict":"approved"}
{"case_id":"l5-overlay-cartons-pairwise-001","worker_output_path":"tests/fixtures/calibration/customs-classification/packaging-cartons/pairwise/case-001-corrupted.json","expected_verdict":"rejected"}
{"case_id":"l5-overlay-gloves-weird-001","worker_output_path":"tests/fixtures/calibration/customs-classification/examination-gloves/weird/case-001.json","expected_verdict":"flagged"}
```

Note: the actual calibration fixture files are created by Phase 6 gold-set beads. This bead's canary references are forward-declared; Phase 6 fixtures must satisfy them before Phase 5 promotion proves out.

Version-binding contract record:

```yaml
# stored alongside skill registry — implementation may use a registry table or YAML index
compat_matrix:
  - worker: skills/supplier-price-enrichment/alibaba@2.0.0
    validator: skills/validation/overlay-customs-classification@1.0.0
    state: compatible
  # ... per source binding
```

Tests to add (TDD §8.5):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-freight_engine-013` | T3 | skill promotion fixture: eBay, Alibaba, 1688, Amazon, PCS production bindings include 6 files; incompatible versions block promotion (overlay-customs-classification is one of the bound validators) |
| `L5-T-freight_engine-018` | T3 | skill journey: same skill at Candidate + Stable stages; candidate cannot become authoritative without canary/shadow gates |

## Tests to Pass

- **`L5-T-freight_engine-013`** (T3) — partial; full closure in `l5-p5-marketplace-fixture-promotion-gate`
- **`L5-T-freight_engine-018`** (T3) — partial; full closure in promotion gate

## Acceptance Criteria

- [ ] `skills/validation/overlay-customs-classification/` has all 6 standard files.
- [ ] Rubric covers automation-sensitive fields per §11.8 redaction taxonomy.
- [ ] Compat matrix records explicit worker_behavior_version × validator_behavior_version pairs.
- [ ] Incompatible-version assertion testable.
- [ ] Both T3 IDs at least partially exercised here.

## Dependencies

- **Blocked by:** `l5-p1-scope-preservation-gate`.
- **Blocks:** `l5-p5-alibaba-maturation`, `l5-p5-1688-maturation`, `l5-p5-amazon-maturation`, `l5-p5-ebay-baseline-binding`.
- **Phase:** 5.

---

### l5-p5-alibaba-maturation

- id: `testproject1-hn05.5.2`
- type: task
- priority: P0
- labels: l5,l5-phase-5,layer-5,marketplace
- parent: `testproject1-hn05.5`
- blocked by: `testproject1-hn05.5.1`

#### Description

# l5-p5-alibaba-maturation

## SOURCE OF TRUTH WARNING

- **Spec**: §1.15.1 (Alibaba delta row), §2.3.1, §2.3.6, §3.3.9, §3.7.2 (PRIMARY)
- **TDD**: `L5-T-freight_engine-009` (PRIMARY), §10 Marketplace Canary Input Shape
- **Strategy**: Phase 5 (parallel after overlay)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Matures `skills/supplier-price-enrichment/alibaba/` from `promotion_stage: candidate` to stable. EXACT deltas vs current 241-line SKILL.md per §1.15 + §2.3.1 + §2.3.6.

## Files to Create/Modify

- **Modify:** `skills/supplier-price-enrichment/alibaba/SKILL.md` (add Chinese retry + Ready-to-Ship filter + source-health canary expansion + anti-bot circuit breaker + validator overlay binding; preserve existing 241 lines)
- **Modify:** `skills/supplier-price-enrichment/alibaba/result.schema.json` (preserve current shape; add fields if needed)
- **Create:** `skills/supplier-price-enrichment/alibaba/checkpoint.schema.json` (if missing)
- **Create:** `skills/supplier-price-enrichment/alibaba/canary.inputs.jsonl` (≥5 cases per bootstrap §7.2)
- **Create:** `skills/supplier-price-enrichment/alibaba/golden.assertions.yaml`
- **Create:** `skills/supplier-price-enrichment/alibaba/validator-rubric.yaml` (binds to `overlay-customs-classification` or marketplace-specific overlay)

## Spec Requirements (verbatim)

**L5-FR-1.15.1 — Existing marketplace skills mature from candidate to decision-grade.**

Alibaba delta row (§1.15 table):
"Chinese retry on zero/accessory/consumer hits; Ready-to-Ship filter bumps `quoteUsabilityConfidence`; source-health canary expansion; anti-bot circuit breaker; validator overlay binding"

**L5-FR-2.3.1 — Alibaba search remains an optimistic-floor signal with stricter recovery.**
- AC1: English query waves run first; Chinese retry runs when English returns zero, accessory, or consumer-item results.
- AC2: Ready-to-Ship listings bump `quoteUsabilityConfidenceBps` only when unit/MOQ/shipping clarity is present.
- AC3: Source-health canary expansion and anti-bot circuit breaker run before production jobs.
- AC4: Alibaba rows keep `price_signal_role = optimistic_floor`.
- AC5: Chinese retry results must pass a back-translation/head-noun compatibility check before supplier quote emission; ambiguous translations become warnings, not quotes.

**L5-FR-2.3.6 — Alibaba maturation preserves the running SearchProfile and output contract.**
- AC1: Alibaba reads every `search_profiles[]` field required by the existing skill, including critical attributes, numeric specs, standards, aliases, exclusions, query waves, and eligibility state; implementations that collapse a profile to only head noun or candidate name fail validation.
- AC2: Alibaba verifies authenticated persistent-profile browser state before the first query; unauthenticated sessions, CAPTCHA blocks, locale/currency drift, page-shape drift, and hard-conflict-zero-survivor cases emit source-specific failure codes and do not fabricate quotes.
- AC3: Alibaba filtering preserves the existing hard-conflict rules for head-noun/alias conformance, required critical attributes, numeric-spec tolerance, exclusion terms, and regulated-goods standards conflicts.
- AC4: Alibaba shortlist/detail behavior preserves max 2 listings per seller, max 2 per query wave, max 1 per normalized title family, and top 3 detail-page enrichment for specs, seller metadata, shipping/Incoterm facts, and compliance evidence tier.
- AC5: Alibaba confidence scoring remains formula-backed in bps for `match_confidence_bps` and `quote_usability_confidence_bps`; Step 03 TDD must assert the current weighted formulas or record a deliberate formula replacement before bead creation.
- AC6: Every Alibaba run emits schema-valid `result.json` with `search_attempts[]` and `supplier_quotes[]` plus `run-manifest.md`; failed batches still emit `supplier_quotes: []`, attempted-search metadata, and non-null `failure_code`.

**L5-FR-3.3.9 — Scenario: Alibaba search with Chinese retry.**
- AC1: English search failure or accessory swamp triggers Chinese retry and produces or records evidence-backed results.

**L5-FR-3.7.2 — Risk: anti-bot detection on Alibaba/1688.**
- AC1: Mitigation is source-health canaries, auth-refresh skills, and circuit breaker.

## Implementation Details

EXACT deltas to add to SKILL.md (preserve current 241 lines; insert delta blocks at appropriate sections):

| Delta | Where in SKILL.md | What it adds |
|-------|-------------------|--------------|
| Chinese retry | query-waves section | When English wave returns 0 hits OR ≥80% accessory/consumer items, trigger Chinese query wave using `aliases_zh` from SearchProfile. Back-translation check required before quote emission. Failure code: `VOCABULARY_MISMATCH_EN_ONLY` (for ambiguous translation). |
| Ready-to-Ship filter | filtering section | "Ready to Ship" listings only bump `quote_usability_confidence_bps` when unit/MOQ/shipping clarity all present. Otherwise treat as standard listing. |
| Source-health canary expansion | preflight section | Canary list expanded to cover 5 product archetypes; Alibaba run gated on canary pass. |
| Anti-bot circuit breaker | preflight + per-run section | After N consecutive CAPTCHA / anti-bot blocks, circuit-breaker opens and Alibaba runs are paused; manual operator clearance reopens. |
| Validator overlay binding | frontmatter | `validator_overlay: validation/overlay-customs-classification` (or marketplace-specific overlay); `worker_behavior_version: 2.0.0` |
| Promotion to stable | frontmatter | `promotion_stage: stable` — only after gold-set canaries + shadow-mode pass (closed by `l5-p5-marketplace-fixture-promotion-gate`) |

Failure codes (Alibaba-specific, all emitted via daemon enum where applicable):

- `ALIBABA_UNAUTH_SESSION`
- `ALIBABA_CAPTCHA_BLOCKED` (existing `SourceCaptchaBlocked` enum reuse)
- `ALIBABA_LOCALE_DRIFT` (existing `LocaleOrCurrencyDrift` enum reuse)
- `ALIBABA_PAGE_SHAPE_DRIFT` (existing `PageShapeDrift` enum reuse)
- `ALIBABA_HARD_CONFLICT_ZERO_SURVIVORS`
- `VOCABULARY_MISMATCH_EN_ONLY` (not new daemon enum; warning-level skill output)

Canary inputs (bootstrap §7.2):

```jsonl
{"case_id":"l5-alibaba-fasteners-001","source_id":"alibaba","category":"fasteners","search_profile":{...full SearchProfile per §10 marketplace canary contract...},"expected_price_signal_role":"optimistic_floor","expected_failure_code":null}
# ... 4 more cases covering: Chinese retry trigger, Ready-to-Ship confidence bump, anti-bot/auth failure, hard-conflict zero survivors
```

Golden assertions cover all 6 §2.3.6 ACs + §10 marketplace canary assertion families: result schema validation, source-specific failure codes, price-signal role, currency/unit normalization, anti-bot/auth-state detection, Chinese retry trigger.

Tests to add (TDD §8.5):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-freight_engine-009` | T3 | Alibaba canary: ≥5 inputs; validates result schema; Chinese retry on English zero/accessory/consumer hits; preserves SearchProfile fields and formulas |

## Tests to Pass

- **`L5-T-freight_engine-009`** (T3)

## Acceptance Criteria

- [ ] All 6 skill directory standard files exist for Alibaba.
- [ ] Chinese retry trigger documented in SKILL.md.
- [ ] Ready-to-Ship filter logic documented (conditional confidence bump).
- [ ] Anti-bot circuit breaker behavior documented + canary case for it.
- [ ] Validator overlay bound; `worker_behavior_version` set.
- [ ] `promotion_stage` remains `candidate` until `l5-p5-marketplace-fixture-promotion-gate` promotes.
- [ ] Existing 241-line SKILL.md not regressed (line count likely grows; the existing behaviors preserved).
- [ ] All 6 §2.3.6 ACs exercised by canary cases.

## Dependencies

- **Blocked by:** `l5-p5-validator-overlay-contract`.
- **Blocks:** `l5-p5-source-health-auth-binding`, `l5-p5-marketplace-fixture-promotion-gate`, `l5-p5-skills-scope-preservation-gate`.
- **Phase:** 5.

## Cross-Phase Promotion Tests

The candidate→stable promotion of this source is gated by two additional test IDs whose primary owner is `l5-p5-marketplace-fixture-promotion-gate` (`hn05.5.8`) but which this maturation bead must also satisfy:

- **`L5-T-freight_engine-013`** (T3, skill promotion fixture): eBay/Alibaba/1688/Amazon/PCS production bindings include the 6-file skill directory standard (`SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`); incompatible worker_behavior_version × validator_behavior_version pairs block promotion.
- **`L5-T-freight_engine-018`** (T3, skill journey fixture): same skill artifact runs at `Candidate` and `Stable` journey stages; candidate output cannot become authoritative without canary + shadow gates passing; stable output is honored; rollback returns a failed version to `candidate`.

This bead's deliverable enters Phase 5's promotion-gate harness via the calibration mirror at `tests/fixtures/calibration/marketplace-search/{source}/{canary.inputs.jsonl, golden.assertions.yaml}`. Both `L5-T-freight_engine-013` and `L5-T-freight_engine-018` must pass before SKILL.md is allowed to flip `promotion_stage: candidate → stable`.

---

### l5-p5-1688-maturation

- id: `testproject1-hn05.5.3`
- type: task
- priority: P0
- labels: l5,l5-phase-5,layer-5,marketplace
- parent: `testproject1-hn05.5`
- blocked by: `testproject1-hn05.5.1`

#### Description

# l5-p5-1688-maturation

## SOURCE OF TRUTH WARNING

- **Spec**: §1.15.1 (1688 delta row), §2.3.2, §3.3.13 (PRIMARY)
- **TDD**: `L5-T-freight_engine-010` (PRIMARY)
- **Strategy**: Phase 5 (parallel after overlay)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Matures `skills/supplier-price-enrichment/1688/` from candidate to stable. EXACT deltas vs current 90-line SKILL.md per §1.15 + §2.3.2.

## Files to Create/Modify

- **Modify:** `skills/supplier-price-enrichment/1688/SKILL.md`
- **Modify:** `skills/supplier-price-enrichment/1688/result.schema.json`
- **Create:** `skills/supplier-price-enrichment/1688/checkpoint.schema.json`
- **Create:** `skills/supplier-price-enrichment/1688/canary.inputs.jsonl`
- **Create:** `skills/supplier-price-enrichment/1688/golden.assertions.yaml`
- **Create:** `skills/supplier-price-enrichment/1688/validator-rubric.yaml`

## Spec Requirements (verbatim)

**L5-FR-1.15.1** 1688 delta row: "Short Chinese-keyword-only clusters (2-4 terms); domestic wholesale floor pricing; auth-refresh/1688 coordination; validator overlay binding"

**L5-FR-2.3.2 — 1688 search is domestic wholesale floor evidence, not regulated-goods execution evidence.**
- AC1: Queries use Chinese-only short clusters of 2-4 terms.
- AC2: Results without explicit compliance/export evidence cannot drive landed-cost decisions for regulated goods.
- AC3: Auth refresh uses `auth-refresh/1688` before production search.
- AC4: 1688 rows keep `price_signal_role = domestic_wholesale_floor`.
- AC5: `VOCABULARY_MISMATCH_EN_ONLY` or equivalent zero-result vocabulary failures require lexicon expansion rather than treating the product as unavailable.

**L5-FR-3.3.13 — Scenario: 1688 Chinese-only wholesale floor with vocabulary mismatch.**
- AC1: Chinese short-cluster search emits domestic-wholesale-floor quotes when vocabulary matches and emits vocabulary-mismatch failure when no Chinese alias can be validated.

## Implementation Details

EXACT deltas to add to SKILL.md (current 90 lines):

| Delta | What it adds |
|-------|--------------|
| Chinese-only short clusters | Query waves use 2-4 Chinese terms; English queries not used. Lexicon expansion (not unavailability) on zero results. |
| Domestic wholesale floor | `price_signal_role = "domestic_wholesale_floor"` constant on every emitted quote; cannot become executable for regulated goods. |
| Auth refresh coordination | Before production search, call `skills/auth-refresh/1688/` (if it exists) or document the manual auth refresh step. Auth state validated before query waves. |
| Validator overlay binding | frontmatter `validator_overlay`, `worker_behavior_version` |

Regulated-goods guard: result-emission layer checks if `category ∈ regulated_goods_list` (fasteners with structural rating, examination gloves, etc.); if so AND no compliance/export evidence in detail page, emit `1688_REGULATED_GOODS_EVIDENCE_MISSING` warning and suppress the supplier_quote (do not emit a quote that would drive landed-cost).

Canary inputs (5 minimum, Chinese-only clusters):

```jsonl
{"case_id":"l5-1688-fasteners-zh-001","source_id":"1688","category":"fasteners","search_profile":{"id":"sp-fasteners-zh-001","head_noun":"螺栓","aliases_zh":["六角螺栓","不锈钢螺栓"],"query_waves_json":[{"wave":0,"strategy":"short_cluster","query_zh":"不锈钢 六角螺栓"}],...},"expected_price_signal_role":"domestic_wholesale_floor"}
# ... 4 more covering: vocabulary mismatch (lexicon expansion path), regulated-goods evidence missing, auth-refresh required, short-cluster pruning to 2 terms
```

Failure codes:

- `VOCABULARY_MISMATCH_EN_ONLY` (or 1688-specific `1688_VOCABULARY_MISMATCH_NO_VALID_ZH_ALIAS`)
- `1688_AUTH_REFRESH_REQUIRED`
- `1688_REGULATED_GOODS_EVIDENCE_MISSING`
- existing daemon enums: `SourceCaptchaBlocked`, `AuthStateStale`, `PageShapeDrift`, `LocaleOrCurrencyDrift`

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-freight_engine-010` | T3 | 1688 canary: ≥5 Chinese-only short-cluster inputs; emits domestic wholesale floor rows; runs auth-refresh; treats vocabulary mismatch as lexicon expansion |

## Tests to Pass

- **`L5-T-freight_engine-010`** (T3)

## Acceptance Criteria

- [ ] All 6 skill directory standard files exist for 1688.
- [ ] Chinese-only short clusters (2-4 terms) enforced in query-wave plan.
- [ ] `price_signal_role = "domestic_wholesale_floor"` on every quote.
- [ ] Auth refresh integration documented (skill or manual).
- [ ] Vocabulary mismatch → lexicon expansion (not unavailability).
- [ ] Regulated-goods guard suppresses quote when compliance evidence absent.
- [ ] `L5-T-freight_engine-010` passes.

## Dependencies

- **Blocked by:** `l5-p5-validator-overlay-contract`.
- **Blocks:** `l5-p5-source-health-auth-binding`, `l5-p5-marketplace-fixture-promotion-gate`, `l5-p5-skills-scope-preservation-gate`.
- **Phase:** 5.

## Cross-Phase Promotion Tests

The candidate→stable promotion of this source is gated by two additional test IDs whose primary owner is `l5-p5-marketplace-fixture-promotion-gate` (`hn05.5.8`) but which this maturation bead must also satisfy:

- **`L5-T-freight_engine-013`** (T3, skill promotion fixture): eBay/Alibaba/1688/Amazon/PCS production bindings include the 6-file skill directory standard (`SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`); incompatible worker_behavior_version × validator_behavior_version pairs block promotion.
- **`L5-T-freight_engine-018`** (T3, skill journey fixture): same skill artifact runs at `Candidate` and `Stable` journey stages; candidate output cannot become authoritative without canary + shadow gates passing; stable output is honored; rollback returns a failed version to `candidate`.

This bead's deliverable enters Phase 5's promotion-gate harness via the calibration mirror at `tests/fixtures/calibration/marketplace-search/{source}/{canary.inputs.jsonl, golden.assertions.yaml}`. Both `L5-T-freight_engine-013` and `L5-T-freight_engine-018` must pass before SKILL.md is allowed to flip `promotion_stage: candidate → stable`.

---

### l5-p5-amazon-maturation

- id: `testproject1-hn05.5.4`
- type: task
- priority: P0
- labels: l5,l5-phase-5,layer-5,marketplace
- parent: `testproject1-hn05.5`
- blocked by: `testproject1-hn05.5.1`, `testproject1-hn05.1.4`

#### Description

# l5-p5-amazon-maturation

## SOURCE OF TRUTH WARNING

- **Spec**: §1.15.1 (Amazon delta row), §2.3.3, §2.3.4 AC2, §3.3.14 (PRIMARY)
- **TDD**: `L5-T-freight_engine-011` (PRIMARY)
- **Strategy**: Phase 5 (parallel after overlay)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Matures `skills/supplier-price-enrichment/amazon/` from candidate to stable + adds source registry entry (registry entry shape from `l5-p1-source-registry-shape`). EXACT deltas vs current 92-line SKILL.md. PA-API 5.0 sunset on 2026-04-30 — browser-first operation is mandatory under §2.3.3 AC2.

## Files to Create/Modify

- **Modify:** `skills/supplier-price-enrichment/amazon/SKILL.md`
- **Modify:** `skills/supplier-price-enrichment/amazon/result.schema.json`
- **Create:** `skills/supplier-price-enrichment/amazon/checkpoint.schema.json`
- **Create:** `skills/supplier-price-enrichment/amazon/canary.inputs.jsonl`
- **Create:** `skills/supplier-price-enrichment/amazon/golden.assertions.yaml`
- **Create:** `skills/supplier-price-enrichment/amazon/validator-rubric.yaml`
- **Modify:** `crates/daemon/src/sources/registry.rs` (production binding for Amazon entry added by `l5-p1-source-registry-shape`)

## Spec Requirements (verbatim)

**L5-FR-1.15.1** Amazon delta row: "Alias families; structured title vocabulary; browser-first strategy after PA-API 5.0 sunset on 2026-04-30; validator overlay binding"

**L5-FR-2.3.3 — Amazon search is browser-first benchmark-ceiling evidence.**
- AC1: Query waves use alias families and structured title vocabulary.
- AC2: PA-API 5.0 is not used after its 2026-04-30 sunset; browser-first operation is required.
- AC3: Amazon rows keep `price_signal_role = benchmark_ceiling`.

**L5-FR-2.3.4 — Marketplace source registration is explicit.**
- AC2: Amazon gains a supplier source entry in `crates/daemon/src/sources/registry.rs` adjacent to Alibaba/1688 supplier registrations.

**L5-FR-3.3.14 — Scenario: Amazon browser-first benchmark ceiling.**
- AC1: Browser-first Amazon search emits benchmark-ceiling GBP quotes, selects a concrete ASIN variant, and rejects ambiguous variant-family pseudo-prices.

## Implementation Details

EXACT deltas to SKILL.md (current 92 lines):

| Delta | What it adds |
|-------|--------------|
| Alias families | Query waves expand alias-family vocabulary (e.g., "screw" → ["screw", "bolt", "machine screw", "wood screw"]). Each alias family is a query wave. |
| Structured title vocabulary | Detail-page title parsed for structured tokens (brand, model, attributes); used to disambiguate variant-family pages. |
| Browser-first strategy | After 2026-04-30 PA-API 5.0 sunset, the skill MUST use browser CDP path; PA-API code paths removed or `disabled` flag set. |
| Concrete ASIN variant | Quote emission requires a concrete ASIN (B0-prefixed identifier); variant-family pseudo-pages rejected with `AMAZON_AMBIGUOUS_VARIANT_FAMILY` failure code. |
| Validator overlay binding | frontmatter set |
| Source registry entry | Production binding in `sources/registry.rs` (shape created by Phase 1; this bead populates worker/validator behavior versions) |

Failure codes:

- `AMAZON_PAAPI_SUNSET_DETECTED`
- `AMAZON_BROWSER_FIRST_REQUIRED`
- `AMAZON_AMBIGUOUS_VARIANT_FAMILY`
- existing daemon enums where applicable

Canary inputs (5 minimum):

```jsonl
{"case_id":"l5-amazon-fasteners-001","source_id":"amazon","category":"fasteners","search_profile":{"head_noun":"hex bolt","noun_aliases_en":["hex bolt","hex screw","hex cap screw"],"query_waves_json":[{"wave":0,"strategy":"alias_family","query_en":"stainless hex bolt"}],...},"expected_price_signal_role":"benchmark_ceiling","expected_failure_code":null}
# ... 4 more covering: alias family expansion, structured title disambiguation, ambiguous variant-family rejection, browser-first only (no PA-API fallback)
```

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-freight_engine-011` | T3 | Amazon canary: ≥5 alias-family inputs; runs browser-first after PA-API sunset; emits benchmark-ceiling rows; rejects ambiguous pseudo-prices |

## Tests to Pass

- **`L5-T-freight_engine-011`** (T3)

## Acceptance Criteria

- [ ] All 6 skill directory standard files exist for Amazon.
- [ ] Browser-first path documented; no PA-API fallback code path active.
- [ ] Alias-family expansion in query waves.
- [ ] Structured title vocabulary used for variant disambiguation.
- [ ] `price_signal_role = "benchmark_ceiling"` constant.
- [ ] `crates/daemon/src/sources/registry.rs` has Amazon production binding (worker/validator behavior versions set).
- [ ] `L5-T-freight_engine-011` passes.

## Dependencies

- **Blocked by:** `l5-p5-validator-overlay-contract`, `l5-p1-source-registry-shape`.
- **Blocks:** `l5-p5-source-health-auth-binding`, `l5-p5-marketplace-fixture-promotion-gate`, `l5-p5-skills-scope-preservation-gate`.
- **Phase:** 5.

## Cross-Phase Promotion Tests

The candidate→stable promotion of this source is gated by two additional test IDs whose primary owner is `l5-p5-marketplace-fixture-promotion-gate` (`hn05.5.8`) but which this maturation bead must also satisfy:

- **`L5-T-freight_engine-013`** (T3, skill promotion fixture): eBay/Alibaba/1688/Amazon/PCS production bindings include the 6-file skill directory standard (`SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`); incompatible worker_behavior_version × validator_behavior_version pairs block promotion.
- **`L5-T-freight_engine-018`** (T3, skill journey fixture): same skill artifact runs at `Candidate` and `Stable` journey stages; candidate output cannot become authoritative without canary + shadow gates passing; stable output is honored; rollback returns a failed version to `candidate`.

This bead's deliverable enters Phase 5's promotion-gate harness via the calibration mirror at `tests/fixtures/calibration/marketplace-search/{source}/{canary.inputs.jsonl, golden.assertions.yaml}`. Both `L5-T-freight_engine-013` and `L5-T-freight_engine-018` must pass before SKILL.md is allowed to flip `promotion_stage: candidate → stable`.

---

### l5-p5-ebay-baseline-binding

- id: `testproject1-hn05.5.5`
- type: task
- priority: P1
- labels: l5,l5-phase-5,layer-5,marketplace
- parent: `testproject1-hn05.5`
- blocked by: `testproject1-hn05.5.1`

#### Description

# l5-p5-ebay-baseline-binding

## SOURCE OF TRUTH WARNING

- **Spec**: §1.15 (eBay remains baseline), §2.3.4, §2.3.5 (PRIMARY)
- **TDD**: `L5-T-customs_classification-008,011`, §10 Marketplace Canary Input Shape (PRIMARY)
- **Strategy**: Phase 5 (eBay baseline alongside Alibaba/1688/Amazon)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

eBay is the existing baseline supplier source (the spec says "eBay remains the existing baseline supplier source"). This bead does NOT mature eBay (already operational); it WIRES the production binding into the L5 validator overlay + canary fixture mirror so the marketplace promotion gate can prove out the full skill directory standard for all 5 production-bound sources (eBay, Alibaba, 1688, Amazon, PCS).

## Files to Create/Modify

- **Modify (if not already at standard):** `skills/supplier-price-enrichment/ebay/` — verify all 6 standard files present; create the missing ones.
- **Modify:** `skills/supplier-price-enrichment/ebay/SKILL.md` (add validator overlay binding if missing)
- **Possibly create:** `tests/fixtures/calibration/marketplace-search/ebay/{canary.inputs.jsonl, golden.assertions.yaml}`

## Spec Requirements (verbatim)

From §1.15 narrative: "Repository baseline verified during validation: `skills/supplier-price-enrichment/{ebay,alibaba,amazon,1688}/SKILL.md` all exist and currently declare `promotion_stage: candidate`. L5 matures Alibaba, 1688, and Amazon because they are the expanded-source deltas; **eBay remains the existing baseline supplier source**."

**L5-FR-2.3.4 — Marketplace source registration is explicit.**
- AC3: Validator overlay binding records worker/validator behavior versions per source.

**L5-FR-2.3.5 — Skill promotion uses the full production directory and validator standard.**
- AC1: eBay, Alibaba, 1688, Amazon, and PCS production bindings include `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, and `validator-rubric.yaml`.

## Implementation Details

Verification step: read existing `skills/supplier-price-enrichment/ebay/SKILL.md` (86 lines). Existing files present:

- `SKILL.md`
- `result.schema.json`

Files to add per §2.3.5:

- `checkpoint.schema.json`
- `canary.inputs.jsonl` (≥5 cases — at minimum: 1 per gold-set category, plus regression cases for any known eBay edge case)
- `golden.assertions.yaml` (assertions matching §10 marketplace canary contract: result schema, source-specific failure codes, price-signal role = `benchmark_ceiling` for eBay, currency/unit normalization)
- `validator-rubric.yaml` (binds to `validation/overlay-customs-classification` or marketplace-specific overlay)

Update SKILL.md frontmatter:

```yaml
validator_overlay: validation/overlay-customs-classification
worker_behavior_version: 1.0.0   # or whatever existing version is
validator_behavior_version: 1.0.0
promotion_stage: stable          # eBay is already operational baseline
```

eBay quote-role: `benchmark_ceiling` (same as Amazon; ceiling not floor).

Canary inputs (5 minimum):

```jsonl
{"case_id":"l5-ebay-fasteners-001","source_id":"ebay","category":"fasteners","search_profile":{...},"expected_price_signal_role":"benchmark_ceiling"}
{"case_id":"l5-ebay-cartons-001","source_id":"ebay","category":"packaging-cartons","search_profile":{...},"expected_price_signal_role":"benchmark_ceiling"}
{"case_id":"l5-ebay-gloves-001","source_id":"ebay","category":"examination-gloves","search_profile":{...},"expected_price_signal_role":"benchmark_ceiling"}
{"case_id":"l5-ebay-zero-results-recovery-001","search_profile":{...low-vocab-match...},"expected_failure_code":"EBAY_ZERO_RESULTS"}
{"case_id":"l5-ebay-currency-normalization-001","search_profile":{...non-GBP listing...},"expected_normalization":"normalized_to_GBP"}
```

Tests to add (TDD §8.2):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-customs_classification-008` | T2 | Alibaba, 1688, Amazon, eBay rows coexist in `supplier_quotes` with source-specific `price_signal_role`, evidence, origin, confidence; quote schema unchanged |
| `L5-T-customs_classification-011` | T1 | every validated category has eBay PLUS at least one of Alibaba/Amazon/1688; floor/ceiling roles never silently become executable quotes |

## Tests to Pass

- **`L5-T-customs_classification-008`** (T2) — partial (eBay row in `supplier_quotes` alongside the 3 matured sources)
- **`L5-T-customs_classification-011`** (T1) — partial (eBay baseline coverage)

## Acceptance Criteria

- [ ] All 6 skill directory standard files exist for eBay.
- [ ] Validator overlay bound.
- [ ] `promotion_stage: stable` (eBay is already operational, not maturing).
- [ ] `price_signal_role = "benchmark_ceiling"` constant.
- [ ] Both test IDs partially exercised (full closure in `l5-p5-marketplace-fixture-promotion-gate`).

## Dependencies

- **Blocked by:** `l5-p5-validator-overlay-contract`.
- **Blocks:** `l5-p5-source-health-auth-binding`, `l5-p5-marketplace-fixture-promotion-gate`, `l5-p5-skills-scope-preservation-gate`.
- **Phase:** 5.

## Cross-Phase Promotion Tests

The candidate→stable promotion of this source is gated by two additional test IDs whose primary owner is `l5-p5-marketplace-fixture-promotion-gate` (`hn05.5.8`) but which this maturation bead must also satisfy:

- **`L5-T-freight_engine-013`** (T3, skill promotion fixture): eBay/Alibaba/1688/Amazon/PCS production bindings include the 6-file skill directory standard (`SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`); incompatible worker_behavior_version × validator_behavior_version pairs block promotion.
- **`L5-T-freight_engine-018`** (T3, skill journey fixture): same skill artifact runs at `Candidate` and `Stable` journey stages; candidate output cannot become authoritative without canary + shadow gates passing; stable output is honored; rollback returns a failed version to `candidate`.

This bead's deliverable enters Phase 5's promotion-gate harness via the calibration mirror at `tests/fixtures/calibration/marketplace-search/{source}/{canary.inputs.jsonl, golden.assertions.yaml}`. Both `L5-T-freight_engine-013` and `L5-T-freight_engine-018` must pass before SKILL.md is allowed to flip `promotion_stage: candidate → stable`.

---

### l5-p5-pcs-scotland-skill

- id: `testproject1-hn05.5.6`
- type: task
- priority: P0
- labels: l5,l5-phase-5,layer-5,pcs,scotland
- parent: `testproject1-hn05.5`
- blocked by: `testproject1-hn05.5.1`, `testproject1-hn05.1.4`

#### Description

# l5-p5-pcs-scotland-skill

## SOURCE OF TRUTH WARNING

- **Spec**: §1.15.2, §3.1.3, §3.3.15 (PRIMARY)
- **TDD**: `L5-T-freight_engine-014` (PRIMARY)
- **Strategy**: Phase 5 (PCS unconditional per operator decision 2026-05-08)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

PCS Scotland procurement ingestion is TRULY NEW. Per operator decision 2026-05-08 recorded in spec Decision 2: IN-SCOPE. Full skill directory standard with 6 files + daemon registry entry (registry shape added by `l5-p1-source-registry-shape`).

## Files to Create/Modify

- **Create:** `skills/procurement-ingestion/pcs/SKILL.md`
- **Create:** `skills/procurement-ingestion/pcs/result.schema.json`
- **Create:** `skills/procurement-ingestion/pcs/checkpoint.schema.json`
- **Create:** `skills/procurement-ingestion/pcs/canary.inputs.jsonl`
- **Create:** `skills/procurement-ingestion/pcs/golden.assertions.yaml`
- **Create:** `skills/procurement-ingestion/pcs/validator-rubric.yaml`
- **Modify:** `crates/daemon/src/sources/registry.rs` (PCS production binding — registry entry shape added by Phase 1; this bead populates worker_behavior_version etc.)
- **Modify:** existing bootstrap doc (already references PCS prereqs in §3 + §4.3 — verify no further edits needed)

## Spec Requirements (verbatim)

**L5-FR-1.15.2 — PCS Scotland ingestion is truly new and in scope.**
- AC1: Phase 5 creates `skills/procurement-ingestion/pcs/` with `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, and `validator-rubric.yaml`.
- AC2: `crates/daemon/src/sources/registry.rs` gains the PCS source entry near existing procurement ingestion sources after `procontract-enrich` and before supplier sources.
- AC3: PCS is not conditional in L5 implementation; the 2026-05-08 operator decision closes the scope question as IN-SCOPE.

**L5-FR-3.1.3 — DoD 3: PCS Scotland ingestion skill operational.**
- AC1: PCS skill directory exists, canary passes, source registry entry is enabled.

**L5-FR-3.3.15 — Scenario: PCS OCDS Web API ingestion.**
- AC1: PCS notice-list and notice-family retrieval creates procurement source rows and passes canary assertions with PCS marked IN-SCOPE.

## Implementation Details

PCS data source: Public Contracts Scotland (PCS). Public OCDS Web API. Documentation reference: standard OCDS (Open Contracting Data Standard) endpoints. No API key required. Existing sibling skills under `skills/procurement-ingestion/` (contracts-finder, fts, procontract-discover, procontract-enrich-tender, procontract-register-interest) provide template patterns.

`SKILL.md` (model on `contracts-finder/SKILL.md`):

```yaml
---
skill_id: procurement-ingestion/pcs
version: 1.0.0
promotion_stage: candidate
interaction_profile: http_public  # OCDS Web API; no auth
auth_requirements: none
result_schema_version: 1
endpoints:
  notice_list: "https://www.publiccontractsscotland.gov.uk/search/search_notices.aspx?api=true"  # confirm exact URL via investigation; PCS exposes JSON via OCDS endpoint
  notice_family: "https://www.publiccontractsscotland.gov.uk/search/show/search_view.aspx?api=true&ID={notice_id}"
validator_overlay: validation/overlay-customs-classification  # or procurement-specific overlay
worker_behavior_version: 1.0.0
validator_behavior_version: 1.0.0
---
```

(Endpoint URLs above are placeholders; the skill author must confirm the exact PCS OCDS endpoint and document it in the SKILL.md. PCS uses publiccontractsscotland.gov.uk; the OCDS export path may differ — verify via `curl` probe.)

`result.schema.json` mirrors the existing `contracts-finder/result.schema.json` shape with PCS-specific fields:

```json
{
  "type": "object",
  "required": ["notices", "checked_at"],
  "properties": {
    "notices": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["notice_id", "publication_date", "title", "buyer_name", "cpv_codes", "lot_count"],
        "properties": {
          "notice_id": { "type": "string" },
          "ocid": { "type": "string" },
          "publication_date": { "type": "string", "format": "date" },
          "title": { "type": "string" },
          "buyer_name": { "type": "string" },
          "buyer_country": { "const": "GB-SCT" },
          "cpv_codes": { "type": "array", "items": { "type": "string" } },
          "lot_count": { "type": "integer" },
          "tender_documents": { "type": "array" }
        }
      }
    },
    "checked_at": { "type": "string", "format": "date-time" }
  }
}
```

Daemon registry entry: `l5-p1-source-registry-shape` already adds the `pcs` entry shape. This bead populates `worker_behavior_version: 1.0.0`, `validator_behavior_version: 1.0.0`, `promotion_stage: candidate`. Once canary passes + shadow runs prove out, `l5-p5-marketplace-fixture-promotion-gate` promotes to stable.

Canary inputs (5 minimum per bootstrap §7.2):

```jsonl
{"case_id":"l5-pcs-notice-list-current","mode":"notice_list","filters":{"date_range_days":7}}
{"case_id":"l5-pcs-notice-detail-fasteners","mode":"notice_detail","notice_id":"<known recent PCS notice with CPV 44530000 fasteners>"}
{"case_id":"l5-pcs-notice-detail-cartons","mode":"notice_detail","notice_id":"<known recent PCS notice for cartons>"}
{"case_id":"l5-pcs-notice-detail-gloves","mode":"notice_detail","notice_id":"<known recent PCS notice for examination gloves>"}
{"case_id":"l5-pcs-notice-list-stale-cassette","mode":"notice_list","cassette_path":"tests/fixtures/cassettes/l5/pcs/notice-list-stale.json"}
```

Golden assertions (per §7.2 PCS row): "result schema, notice-list retrieval, notice-family retrieval, OCDS fields, source registry binding".

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-freight_engine-014` | T3 | PCS canary: `skills/procurement-ingestion/pcs/` exists with full 6-file standard; retrieves PCS OCDS notice-list/family data; registry places it near procurement sources |

## Tests to Pass

- **`L5-T-freight_engine-014`** (T3)

## Acceptance Criteria

- [ ] All 6 skill directory standard files exist for PCS.
- [ ] PCS OCDS endpoint URLs verified live (or documented as TBD investigation if endpoint moved).
- [ ] `crates/daemon/src/sources/registry.rs` PCS entry has worker/validator behavior versions populated.
- [ ] Canary inputs cover all 3 gold-set categories + zero-result recovery + cassette replay.
- [ ] `buyer_country = "GB-SCT"` constant in result schema for Scotland scope.
- [ ] `L5-T-freight_engine-014` passes.
- [ ] No reopening of the 2026-05-08 IN-SCOPE decision anywhere in this bead's output.

## Dependencies

- **Blocked by:** `l5-p1-source-registry-shape`, `l5-p5-validator-overlay-contract`.
- **Blocks:** `l5-p5-source-health-auth-binding`, `l5-p5-marketplace-fixture-promotion-gate`, `l5-p5-skills-scope-preservation-gate`.
- **Phase:** 5.

## Cross-Phase Promotion Tests

The candidate→stable promotion of this source is gated by two additional test IDs whose primary owner is `l5-p5-marketplace-fixture-promotion-gate` (`hn05.5.8`) but which this maturation bead must also satisfy:

- **`L5-T-freight_engine-013`** (T3, skill promotion fixture): eBay/Alibaba/1688/Amazon/PCS production bindings include the 6-file skill directory standard (`SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`); incompatible worker_behavior_version × validator_behavior_version pairs block promotion.
- **`L5-T-freight_engine-018`** (T3, skill journey fixture): same skill artifact runs at `Candidate` and `Stable` journey stages; candidate output cannot become authoritative without canary + shadow gates passing; stable output is honored; rollback returns a failed version to `candidate`.

This bead's deliverable enters Phase 5's promotion-gate harness via the calibration mirror at `tests/fixtures/calibration/marketplace-search/{source}/{canary.inputs.jsonl, golden.assertions.yaml}`. Both `L5-T-freight_engine-013` and `L5-T-freight_engine-018` must pass before SKILL.md is allowed to flip `promotion_stage: candidate → stable`.

---

### l5-p5-source-health-auth-binding

- id: `testproject1-hn05.5.7`
- type: task
- priority: P1
- labels: l5,l5-phase-5,layer-5,source-health
- parent: `testproject1-hn05.5`
- blocked by: `testproject1-hn05.5.6`, `testproject1-hn05.5.4`, `testproject1-hn05.5.3`, `testproject1-hn05.5.2`, `testproject1-hn05.1.5`

#### Description

# l5-p5-source-health-auth-binding

## SOURCE OF TRUTH WARNING

- **Spec**: §1.15.1 (anti-bot circuit breaker, auth-refresh), §3.7.2, §3.10.2 (PRIMARY)
- **TDD**: `L5-T-freight_engine-016` (PRIMARY)
- **Strategy**: Phase 5 (binds after source-specific maturation)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Source-health canary cron (registered by `l5-p1-cron-definitions`) needs an actual canary skill to execute. This bead creates the source-health canary skill and binds it to the 7 covered sources (Alibaba, 1688, Amazon, PCS, SeaRates, Freightos, TARIC). Auth-refresh skills for 1688 (and Alibaba if needed) wire alongside.

## Files to Create/Modify

- **Create:** `skills/source-health/l5-marketplace-commercial-intelligence/SKILL.md`
- **Create:** `skills/source-health/l5-marketplace-commercial-intelligence/result.schema.json`
- **Create:** `skills/source-health/l5-marketplace-commercial-intelligence/checkpoint.schema.json`
- **Create:** `skills/source-health/l5-marketplace-commercial-intelligence/canary.inputs.jsonl`
- **Create:** `skills/source-health/l5-marketplace-commercial-intelligence/golden.assertions.yaml`
- **Create:** `skills/source-health/l5-marketplace-commercial-intelligence/validator-rubric.yaml`
- **Possibly create:** `skills/auth-refresh/1688/` standard directory if it does not already exist (1688 maturation references it; this bead is the implementer)

## Spec Requirements (verbatim)

**L5-FR-3.7.2 — Risk: anti-bot detection on Alibaba/1688.**
- AC1: Mitigation is source-health canaries, auth-refresh skills, and circuit breaker.

**L5-FR-3.10.2 — L5 implements ADR-001 skill-based data retrieval for added sources.**
- AC1: Alibaba, 1688, Amazon, PCS, Freightos, SeaRates, TARIC, HMRC, and FX inputs use skills or public/free endpoints, not external commercial connector accounts.

## Implementation Details

Source-health canary skill: invoked daily by `MARKETPLACE_SOURCE_HEALTH_CANARY_CRON` (cron expression `0 0 5 * * *` from Phase 1). For each of 7 sources, runs a lightweight health check (1 canary input per source) and emits a per-source health verdict.

`SKILL.md`:

```yaml
---
skill_id: source-health/l5-marketplace-commercial-intelligence
version: 1.0.0
promotion_stage: candidate
interaction_profile: contract
model_profile: tier_1_lightweight  # cheap model — health checks are deterministic mostly
auth_requirements: none  # the canary itself runs no privileged action; underlying source skills may need their own auth
covered_sources:
  - alibaba
  - 1688
  - amazon
  - pcs
  - searates
  - freightos
  - taric
validator_overlay: none  # source-health is meta-validation; no separate overlay
worker_behavior_version: 1.0.0
---
```

Result schema:

```json
{
  "type": "object",
  "required": ["checked_at", "per_source_health"],
  "properties": {
    "checked_at": { "type": "string", "format": "date-time" },
    "per_source_health": {
      "type": "object",
      "patternProperties": {
        "^(alibaba|1688|amazon|pcs|searates|freightos|taric)$": {
          "type": "object",
          "required": ["status", "evidence_path"],
          "properties": {
            "status": { "enum": ["healthy", "degraded", "blocked", "auth_required", "unknown"] },
            "evidence_path": { "type": "string" },
            "failure_code": { "type": ["string", "null"] }
          }
        }
      }
    }
  }
}
```

Per-source canary case: 1 minimal canonical case per source (head noun + standard attrs); checks that source returns SOMETHING parseable, not necessarily good quotes. Failure modes mapped to existing daemon enums (`SourceCaptchaBlocked`, `AuthStateStale`, `PageShapeDrift`, etc.).

Tests to add (TDD §8.5):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-freight_engine-016` | T2 | marketplace source-health canary cron exists and exercises Alibaba/1688/Amazon/PCS/SeaRates/Freightos/TARIC readiness without paid API auth checks |

The cron-existence half is closed by `l5-p1-cron-definitions`; this bead closes the SKILL existence half + the canary execution half.

## Tests to Pass

- **`L5-T-freight_engine-016`** (T2) — full closure

## Acceptance Criteria

- [ ] All 6 skill directory files exist for source-health/l5-marketplace-commercial-intelligence.
- [ ] Result schema covers 7 sources.
- [ ] No paid API auth check anywhere in canary cases.
- [ ] Auth-refresh/1688 skill exists if 1688 maturation references it (otherwise document the manual fallback).
- [ ] `L5-T-freight_engine-016` passes.

## Dependencies

- **Blocked by:** `l5-p1-cron-definitions`, `l5-p5-alibaba-maturation`, `l5-p5-1688-maturation`, `l5-p5-amazon-maturation`, `l5-p5-pcs-scotland-skill`.
- **Blocks:** `l5-p5-marketplace-fixture-promotion-gate`, `l5-p5-skills-scope-preservation-gate`.
- **Phase:** 5.

---

### l5-p5-marketplace-fixture-promotion-gate

- id: `testproject1-hn05.5.8`
- type: task
- priority: P0
- labels: l5,l5-phase-5,layer-5,promotion
- parent: `testproject1-hn05.5`
- blocked by: `testproject1-hn05.5.6`, `testproject1-hn05.5.4`, `testproject1-hn05.5.3`, `testproject1-hn05.5.2`, `testproject1-hn05.5.7`, `testproject1-hn05.5.5`

#### Description

# l5-p5-marketplace-fixture-promotion-gate

## SOURCE OF TRUTH WARNING

- **Spec**: §1.15.1, §2.3.4, §2.3.5, §2.4.3 (PRIMARY)
- **TDD**: `L5-T-customs_classification-008,011`, `L5-T-freight_engine-013,018`, §10 Marketplace Canary Input Shape (PRIMARY)
- **Strategy**: Phase 5 (promotion after source-specific beads)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Closes the candidate→stable promotion for all marketplace sources via canary + shadow run gates. Promotes Alibaba, 1688, Amazon from `candidate` to `stable` only after canary + shadow runs pass. Mirrors per-source canary fixtures into the calibration mirror at `tests/fixtures/calibration/marketplace-search/{alibaba,1688,amazon,pcs}/` per TDD §10.

## Files to Create/Modify

- **Create:** `tests/fixtures/calibration/marketplace-search/alibaba/{canary.inputs.jsonl, golden.assertions.yaml}`
- **Create:** `tests/fixtures/calibration/marketplace-search/1688/{canary.inputs.jsonl, golden.assertions.yaml}`
- **Create:** `tests/fixtures/calibration/marketplace-search/amazon/{canary.inputs.jsonl, golden.assertions.yaml}`
- **Create:** `tests/fixtures/calibration/marketplace-search/pcs/{canary.inputs.jsonl, golden.assertions.yaml}`
- **Modify (after gate passes):** Each maturation source SKILL.md `promotion_stage: candidate → stable`

## Spec Requirements (verbatim)

**L5-FR-1.15.1 AC2:** Phase 5 bead scope is limited to the listed deltas and promotion; no speculative gap-fill work is added.

**L5-FR-2.3.5 — Skill promotion uses the full production directory and validator standard.**
- AC2: Stable promotion records compatible `worker_behavior_version` and `validator_behavior_version`; incompatible versions block promotion rather than silently recalibrating.

**L5-FR-2.4.3 — `supplier_quotes` carries additional marketplace rows.**
- AC1: Alibaba, 1688, and Amazon quotes join existing eBay quotes.
- AC2: `priceSignalRole` values are Amazon/eBay `benchmark_ceiling`, Alibaba `optimistic_floor`, 1688 `domestic_wholesale_floor`.

## Implementation Details

Per TDD §10, calibration mirror directory shape:

```
tests/fixtures/calibration/marketplace-search/
  alibaba/{canary.inputs.jsonl, golden.assertions.yaml}
  1688/{canary.inputs.jsonl, golden.assertions.yaml}
  amazon/{canary.inputs.jsonl, golden.assertions.yaml}
  pcs/{canary.inputs.jsonl, golden.assertions.yaml}
```

Each `canary.inputs.jsonl` uses one JSON object per line. Each JSONL row has a stable `case_id` that keys the `golden.assertions.yaml` assertion block.

JSONL row shape (§10):

```json
{
  "case_id": "l5-alibaba-fasteners-001",
  "source_id": "alibaba",
  "category": "fasteners",
  "search_profile": {
    "id": "sp-fasteners-001",
    "product_candidate_id": "pc-fasteners-001",
    "head_noun": "hex bolt",
    "candidate_type": "standard_commodity",
    "critical_attributes_json": {"material": {"value": "stainless steel", "criticality": "required"}},
    "numeric_specs_json": {"diameter_mm": {"nominal": 8, "min": 7.8, "max": 8.2, "unit": "mm", "criticality": "required"}},
    "standards_json": ["DIN 933"],
    "noun_aliases_en": ["hex bolt", "hex screw"],
    "aliases_zh": ["六角螺栓"],
    "query_waves_json": [{"wave": 0, "strategy": "exact_anchor", "query_en": "stainless hex bolt"}],
    "search_eligibility_state": "auto_search"
  },
  "expected_price_signal_role": "optimistic_floor",
  "expected_failure_code": null
}
```

Golden assertions cover §10 list: result schema validation, source-specific failure codes, price-signal role, currency/unit normalization, anti-bot/auth-state detection, Chinese retry trigger for Alibaba, Chinese-only enforcement for 1688, alias-family expansion for Amazon, PCS OCDS notice shape.

Promotion gate logic:

1. Run canaries via skill runner.
2. Validate result schema against `skills/<src>/result.schema.json`.
3. Match against `golden.assertions.yaml`.
4. Run shadow mode: candidate skill output compared against existing baseline output for same input; assert no regressions.
5. Bind worker_behavior_version × validator_behavior_version compatible.
6. Promote SKILL.md `promotion_stage: candidate → stable` ONLY when 1-5 all pass.
7. Failed promotion: candidate stays at `candidate`; failed version returns to `candidate` (rollback path).

Tests to add (TDD §8.5):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-freight_engine-013` | T3 | skill promotion: 5 production-bound skills (eBay, Alibaba, 1688, Amazon, PCS) include 6-file standard; incompatible versions block promotion |
| `L5-T-freight_engine-018` | T3 | skill journey: same skill at Candidate + Stable; candidate cannot become authoritative without canary/shadow gates; stable honored; rollback returns failed version to candidate |

Plus (TDD §8.2):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-customs_classification-008` | T2 | Alibaba/1688/Amazon/eBay rows coexist in `supplier_quotes` with source-specific `price_signal_role`, evidence, origin, confidence |
| `L5-T-customs_classification-011` | T1 | every validated category has eBay + at least one of Alibaba/Amazon/1688 |

## Tests to Pass

- **`L5-T-freight_engine-013`** (T3) — full closure
- **`L5-T-freight_engine-018`** (T3) — full closure
- **`L5-T-customs_classification-008`** (T2) — full closure
- **`L5-T-customs_classification-011`** (T1) — full closure

## Acceptance Criteria

- [ ] 4 calibration mirror directories exist with `canary.inputs.jsonl` + `golden.assertions.yaml`.
- [ ] All marketplace skill canaries pass (Alibaba, 1688, Amazon, eBay, PCS).
- [ ] Shadow-run comparison shows no regressions for Alibaba/1688/Amazon.
- [ ] Worker/validator behavior version compatibility checked before promotion.
- [ ] Alibaba/1688/Amazon `promotion_stage: stable` after gate passes.
- [ ] `supplier_quotes` table populated with rows from all 4 marketplace sources with correct `price_signal_role`.
- [ ] All 4 listed test IDs pass.

## Dependencies

- **Blocked by:** all source-specific Phase 5 beads (`l5-p5-alibaba-maturation`, `l5-p5-1688-maturation`, `l5-p5-amazon-maturation`, `l5-p5-ebay-baseline-binding`, `l5-p5-pcs-scotland-skill`, `l5-p5-source-health-auth-binding`).
- **Blocks:** `l5-p5-skills-scope-preservation-gate`.
- **Phase:** 5.

---

### l5-p5-skills-scope-preservation-gate

- id: `testproject1-hn05.5.9`
- type: task
- priority: P0
- labels: l5,l5-phase-5,layer-5,review-gate
- parent: `testproject1-hn05.5`
- blocked by: `testproject1-hn05.5.8`, `testproject1-hn05.5.6`, `testproject1-hn05.5.4`, `testproject1-hn05.5.3`, `testproject1-hn05.5.2`, `testproject1-hn05.5.1`, `testproject1-hn05.5.7`, `testproject1-hn05.5.5`

#### Description

# l5-p5-skills-scope-preservation-gate

## SOURCE OF TRUTH WARNING

- **Spec**: §1.15, §2.3, §3.1.1-3.1.3 (PRIMARY)
- **TDD**: `L5-T-freight_engine-009..014,016,018`, `L5-T-customs_classification-008,011` (PRIMARY)
- **Strategy**: Phase 5 gate

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Phase 5 review gate. Verifies all marketplace + PCS work integrated. Closes the candidate→stable promotion proof and ensures floor/ceiling quote-role semantics are preserved through the L5→L6 readiness gate.

## Gate Criteria (strategy §Phase 5)

1. Alibaba/1688/Amazon/eBay/PCS satisfy the 6-file skill directory standard.
2. Alibaba/1688/Amazon move from candidate to stable only through canary, shadow/promotion, validator gates.
3. PCS canary proves Scotland ingestion without reopening 2026-05-08 decision.
4. Source-health canary cron covers marketplace + CDP dependency readiness.
5. Quote-role semantics remain explicit; floor/ceiling rows do not silently become executable wholesale quotes.

## Implementation Details

Verification checklist:

```bash
# 6-file standard for all 5 production-bound sources
for src in alibaba 1688 amazon ebay; do
  for f in SKILL.md result.schema.json checkpoint.schema.json canary.inputs.jsonl golden.assertions.yaml validator-rubric.yaml; do
    test -f "skills/supplier-price-enrichment/$src/$f" || exit 1
  done
done
for f in SKILL.md result.schema.json checkpoint.schema.json canary.inputs.jsonl golden.assertions.yaml validator-rubric.yaml; do
  test -f "skills/procurement-ingestion/pcs/$f" || exit 1
done

# Stable promotion verified
for src in alibaba 1688 amazon; do
  grep -q "^promotion_stage: stable$" "skills/supplier-price-enrichment/$src/SKILL.md" || exit 1
done

# Calibration mirror exists
for src in alibaba 1688 amazon pcs; do
  test -f "tests/fixtures/calibration/marketplace-search/$src/canary.inputs.jsonl" || exit 1
  test -f "tests/fixtures/calibration/marketplace-search/$src/golden.assertions.yaml" || exit 1
done

# Source-health skill exists
test -d "skills/source-health/l5-marketplace-commercial-intelligence" || exit 1

# Test IDs
bun test packages/intelligence/src/logistics/freight-engine.test.ts
for i in 009 010 011 012 013 014 016 018; do
  bun test packages/intelligence/src/logistics/freight-engine.test.ts 2>&1 | rg -q "L5-T-freight_engine-$i" || exit 1
done

bun test packages/intelligence/src/customs/customs-classification.test.ts
for i in 008 011; do
  bun test packages/intelligence/src/customs/customs-classification.test.ts 2>&1 | rg -q "L5-T-customs_classification-$i" || exit 1
done

# Floor-ceiling quote-role integrity
rg -q '"price_signal_role":\s*"optimistic_floor"' skills/supplier-price-enrichment/alibaba/canary.inputs.jsonl
rg -q '"price_signal_role":\s*"domestic_wholesale_floor"' skills/supplier-price-enrichment/1688/canary.inputs.jsonl
rg -q '"price_signal_role":\s*"benchmark_ceiling"' skills/supplier-price-enrichment/amazon/canary.inputs.jsonl
rg -q '"price_signal_role":\s*"benchmark_ceiling"' skills/supplier-price-enrichment/ebay/canary.inputs.jsonl
```

## Tests to Pass

- **`L5-T-freight_engine-009..014,016,018`** (8 IDs)
- **`L5-T-customs_classification-008,011`** (2 IDs)

## Acceptance Criteria

- [ ] All 10 listed test IDs pass.
- [ ] All 5 production-bound source skills + source-health skill have full 6-file standard.
- [ ] Alibaba/1688/Amazon at `promotion_stage: stable`.
- [ ] eBay + PCS bound; eBay stable; PCS promotion stage per gate outcome.
- [ ] Quote-role constants documented and enforced in canary data.
- [ ] Phase 5 roll-back conditions NOT triggered.

## Dependencies

- **Blocked by:** all 8 prior Phase 5 task beads.
- **Blocks:** `l5-p6-final-dod-readiness-gate`.
- **Phase:** 5 (final).

---

## Phase 6 Task Beads

### l5-p6-accuracy-harness-module

- id: `testproject1-hn05.6.1`
- type: task
- priority: P0
- labels: accuracy,l5,l5-phase-6,layer-5
- parent: `testproject1-hn05.6`
- blocked by: `testproject1-hn05.4.13`

#### Description

# l5-p6-accuracy-harness-module

## SOURCE OF TRUTH WARNING

- **Spec**: §3.4.1, §3.4.2, §3.4.3, §3.7.3 (PRIMARY)
- **TDD**: `L5-T-customs_classification-028..032,035`, §11.6 (Accuracy Harness Metric Contract), §9.5 (Mutation Taxonomy) (PRIMARY)
- **Strategy**: Phase 6 (accuracy module first; gold-set corpora depend on it)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Owns `packages/intelligence/src/customs/accuracy.ts` — the SINGLE accuracy file (per §1.16 table: "1 file" under `customs/accuracy.ts`). Evidence-tiered metric scoring (gold/silver/bronze), abstention-positive-class, catastrophic understatement, unsafe auto-classification, Wilson lower bounds, mutation-family sweeps.

## Files to Create/Modify

- **Create:** `packages/intelligence/src/customs/accuracy.ts`
- **Modify:** `packages/intelligence/src/customs/customs-classification.test.ts` (add `L5-T-customs_classification-028..032,035`)

## Spec Requirements (verbatim)

**L5-FR-3.4.1 — Classification accuracy is scored by evidence tier.**
- AC1: Gold labels are ATR outcomes, actual import declarations, or broker-confirmed family with no later amendment.
- AC2: Silver labels require two independent internal reviewers with documented reasoning.
- AC3: Bronze labels are best-effort tender-doc labels and cannot certify auto-classification safety alone.

**L5-FR-3.4.2 — Accuracy metrics include abstention and catastrophic-error measures.**
- AC1: Metrics include duty-outcome accuracy, trade-remedy yes/no, additional-code required yes/no, supplementary-unit required yes/no, review-trigger recall, catastrophic understatement rate, unsafe auto-classification rate, and abstention as a positive class.
- AC2: Gold-set categories are exactly fasteners, packaging-cartons, and examination-gloves per 2026-05-08 operator decision.

**L5-FR-3.4.3 — Evidence-tiered metrics gate category production.**
- AC1: Bronze-only labels cannot satisfy auto-classification safety gates for a category.
- AC2: Gold and silver metric reports must separate correct-abstention successes from wrong-code successes and failures.

**L5-FR-3.7.3 — Risk: customs classification wrong for complex products.**
- AC1: Mitigation is multi-source recall, conservative defaults, abstention, broker escalation.

## Implementation Details

§11.6 Accuracy Harness Metric Contract (verbatim output shape):

```json
{
  "category": "fasteners",
  "label_tier_counts": {"gold": 12, "silver": 18, "bronze": 10},
  "duty_outcome_accuracy_bps": 9200,
  "trade_remedy_yes_no_bps": 10000,
  "additional_code_required_yes_no_bps": 9500,
  "supplementary_unit_required_yes_no_bps": 9400,
  "review_trigger_recall_bps": 9800,
  "catastrophic_understatement_rate_bps": 0,
  "unsafe_auto_classification_rate_bps": 0,
  "correct_abstention_count": 4,
  "wrong_code_success_count": 0,
  "production_gate": "eligible"
}
```

Module API:

```ts
import type { TenDigitResolutionResult } from "./ten-digit-resolution.js";
import type { MeasureScanResult } from "./measure-scanner.js";

export type LabelTier = "gold" | "silver" | "bronze";

export interface AccuracyCase {
  caseId: string;
  category: "fasteners" | "packaging-cartons" | "examination-gloves";
  labelTier: LabelTier;
  groundTruth: {
    expectedUk10: string;
    expectedDutyRange: { minPence: number; maxPence: number };
    expectedTradeRemedy: boolean;
    expectedAdditionalCodeRequired: boolean;
    expectedSupplementaryUnitRequired: boolean;
    expectedDecision: "auto_pass" | "review_required" | "broker_required";
  };
  systemOutput: {
    selectedUk10: string;
    dutyPence: number;
    tradeRemedyActive: boolean;
    additionalCodeRequired: boolean;
    supplementaryUnitRequired: boolean;
    decisionGrade: "auto_pass" | "review_required" | "broker_required";
    abstained: boolean;
  };
  mutationFamily: "clean" | "weird" | "mutated" | "pairwise" | "source_identity" | "page_kind" | "currency_locale" | "unit_basis" | "product_identity" | "warning_omission" | "count_mismatch" | "fabricated_evidence" | "continuation_integrity";
  pairId?: string;          // for pairwise corpus
}

export interface AccuracyReport {
  category: string;
  labelTierCounts: { gold: number; silver: number; bronze: number };
  dutyOutcomeAccuracyBps: number;
  tradeRemedyYesNoBps: number;
  additionalCodeRequiredYesNoBps: number;
  supplementaryUnitRequiredYesNoBps: number;
  reviewTriggerRecallBps: number;
  catastrophicUnderstatementRateBps: number;
  unsafeAutoClassificationRateBps: number;
  correctAbstentionCount: number;
  wrongCodeSuccessCount: number;
  productionGate: "eligible" | "ineligible_bronze_only" | "ineligible_thresholds" | "ineligible_catastrophic";
  wilsonLowerBounds: {
    reviewTriggerRecallBps: number;
    tradeRemedyYesNoBps: number;
    additionalCodeRequiredYesNoBps: number;
    supplementaryUnitRequiredYesNoBps: number;
  };
  ruleOfThreeUpperBounds: {
    catastrophicUnderstatementRateBps: number;
    unsafeAutoClassificationRateBps: number;
  };
}

export function scoreCategory(cases: AccuracyCase[]): AccuracyReport;
```

Production gate thresholds (per §11.6):

- `review_trigger_recall` Wilson lower bound ≥ 9500 bps
- `trade_remedy_yes_no`, `additional_code_required_yes_no`, `supplementary_unit_required_yes_no` Wilson lower bounds ≥ 9000 bps
- observed `catastrophic_understatement_rate_bps = 0`
- observed `unsafe_auto_classification_rate_bps = 0`
- rule-of-three upper bound for catastrophic + unsafe rates ≤ 300 bps

Catastrophic understatement: `predicted_max_duty < labeled_duty by >= 1000 bps of customs value OR >= GBP 100 (10000 pence), whichever is lower`.

Abstention positive class: correct abstention (system declines / emits blocker on a hard-negative case) counts as success, NOT failure. Fabricated actionable estimate on hard-negative counts as failure.

Mutation taxonomy (TDD §9.5 + bootstrap §7.1): clean, weird, mutated, pairwise + 9 audit overlays (source_identity, page_kind, currency_locale, unit_basis, product_identity, warning_omission, count_mismatch, fabricated_evidence, continuation_integrity).

Customs-specific mutations: HS6 perturbation, exporter swap, country-of-origin swap, currency swap, packaging weight perturbation, supplementary-unit removal, Incoterm swap, additional-code evidence removal, VAT relief evidence removal, TARIC/HMRC disagreement injection.

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-customs_classification-028` | T3 | gold-tier labels reported separately; gate auto-classification safety |
| `L5-T-customs_classification-029` | T3 | silver-tier labels require 2 independent reviewers; separated from gold |
| `L5-T-customs_classification-030` | T3 | bronze-only labels cannot satisfy production auto-classification safety |
| `L5-T-customs_classification-031` | T3 | abstention scored as positive class on hard negatives + insufficient evidence; fabricated actionable estimates fail |
| `L5-T-customs_classification-032` | T3 | metrics include all 8 measures; catastrophic understatement + unsafe auto-classification incremented on the defined fixture |
| `L5-T-customs_classification-035` | T3 | mutation taxonomy covers `clean/weird/mutated/pairwise` + 9 audit overlays; cross-model audit, override feedback, Wilson lower-bound, rule-of-three, demotion-threshold calculations all present |

## Tests to Pass

- All 6 listed test IDs (`L5-T-customs_classification-028..032,035`).

## Acceptance Criteria

- [ ] `accuracy.ts` exists as the 1 customs accuracy file.
- [ ] `AccuracyReport` output matches §11.6 contract verbatim.
- [ ] Production gate logic enforces Wilson + rule-of-three bounds.
- [ ] Bronze-only labels cannot promote a category.
- [ ] Abstention counted as positive class.
- [ ] Mutation taxonomy covers all 13 families (4 base + 9 audit overlays).
- [ ] All 6 listed test IDs pass.

## Dependencies

- **Blocked by:** `l5-p4-customs-scope-preservation-gate`.
- **Blocks:** `l5-p6-goldset-fasteners-corpus`, `l5-p6-goldset-packaging-cartons-corpus`, `l5-p6-goldset-examination-gloves-corpus`.
- **Phase:** 6.

---

### l5-p6-comparison-adapters

- id: `testproject1-hn05.6.2`
- type: task
- priority: P1
- labels: l5,l5-phase-6,layer-5,validation
- parent: `testproject1-hn05.6`
- blocked by: `testproject1-hn05.4.13`

#### Description

# l5-p6-comparison-adapters

## SOURCE OF TRUTH WARNING

- **Spec**: §1.16.1 AC2, §1.16.2 (PRIMARY)
- **TDD**: `L5-T-freight_engine-017` (PRIMARY)
- **Strategy**: Phase 6 (parallel after customs stable)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Creates the 3 fixture-only comparison adapters under `packages/intelligence/src/validation/`. Dutify/Zonos/Avalara are DECLINED for L5 paid accounts — these adapters use checked-in fixtures or captured samples only. They exist for comparison reporting against external industry references.

## Files to Create/Modify

- **Create:** `packages/intelligence/src/validation/dutify-comparison.ts`
- **Create:** `packages/intelligence/src/validation/zonos-comparison.ts`
- **Create:** `packages/intelligence/src/validation/avalara-comparison.ts`
- **Create:** `tests/fixtures/l5/comparison/{dutify,zonos,avalara}/sample-outputs.json`

## Spec Requirements (verbatim)

**L5-FR-1.16.1 — L5 adds customs, FX, logistics, validation, and accuracy modules under `packages/intelligence`.**
- AC2: Validation comparison adapters remain comparison-only and use fixtures or captured samples, with no external vendor credentials.

**L5-FR-1.16.2 — Intelligence package file counts are fixed for bead planning.**
- AC1: The package additions total exactly 19 files: 6 customs, 4 FX, 5 logistics, 3 validation, and 1 accuracy module.

## Implementation Details

Each adapter exports a typed `compare(opts: { l5Output: LandedCostEstimate; externalSampleFixturePath: string }): ComparisonReport` API. The output:

```ts
export interface ComparisonReport {
  adapter: "dutify" | "zonos" | "avalara";
  l5Output: { dutyPence: number; vatPence: number; freightPence: number; totalPence: number };
  externalOutput: { dutyPence: number; vatPence: number; freightPence: number; totalPence: number };
  divergenceBps: { duty: number; vat: number; freight: number; total: number };
  notes: string[];
}
```

Comparison logic: load the external sample JSON fixture, parse it into the same field schema, compute relative divergence per component (bps). Each adapter file is ~30-60 lines of code; no business logic; no live API call.

Fixture samples must be captured (curated) — NOT generated. Document the capture date and the input scenario each sample represents.

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-freight_engine-017` | T1 (package inventory) | 19 package files present: 6 customs + 4 FX + 5 logistics + 3 validation + 1 accuracy; comparison adapters use fixtures/captured samples only |

This bead alone does not own `L5-T-freight_engine-017` (the package inventory test); it CONTRIBUTES the 3 validation files. Full inventory closure lives in `l5-p6-final-dod-readiness-gate` once all 19 files are present.

## Tests to Pass

- **`L5-T-freight_engine-017`** (T1) — partial; validation/ files contribute.

## Acceptance Criteria

- [ ] 3 validation adapter files exist.
- [ ] No vendor API key reference in any file.
- [ ] Fixture samples checked in; capture date documented.
- [ ] Output shape matches `ComparisonReport`.
- [ ] `bun run build` passes.

## Dependencies

- **Blocked by:** `l5-p4-customs-scope-preservation-gate` (need decision-grade L5 output to compare against).
- **Blocks:** `l5-p6-final-dod-readiness-gate`.
- **Phase:** 6.

---

### l5-p6-goldset-fasteners-corpus

- id: `testproject1-hn05.6.3`
- type: task
- priority: P0
- labels: gold-set,l5,l5-phase-6,layer-5
- parent: `testproject1-hn05.6`
- blocked by: `testproject1-hn05.6.1`

#### Description

# l5-p6-goldset-fasteners-corpus

## SOURCE OF TRUTH WARNING

- **Spec**: §1.2.1, §3.1.12, §3.3.18, Decision 4 (operator 2026-05-08) (PRIMARY)
- **TDD**: `L5-T-customs_classification-027,028,029,030,031,032,035` (PRIMARY), §11.6 (minimum corpus per category)
- **Strategy**: Phase 6 (after accuracy harness)
- **Bootstrap**: §7.1 Gold-Set Categories

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Builds the fasteners gold-set calibration corpus. Fasteners are the REMEDY-ACTIVE category (steel AD/CVD measures, aluminium exporter-specific rates). This corpus exercises the trade-remedy paths heavily.

## Files to Create/Modify

- **Create:** `tests/fixtures/calibration/customs-classification/fasteners/canary.inputs.jsonl`
- **Create:** `tests/fixtures/calibration/customs-classification/fasteners/golden.assertions.yaml`
- **Create:** `tests/fixtures/calibration/customs-classification/fasteners/metadata.json`
- **Create:** `tests/fixtures/calibration/customs-classification/fasteners/manifest.md`
- **Create:** subdirectories for mutation families:
  - `tests/fixtures/calibration/customs-classification/fasteners/clean/`
  - `tests/fixtures/calibration/customs-classification/fasteners/weird/`
  - `tests/fixtures/calibration/customs-classification/fasteners/mutated/`
  - `tests/fixtures/calibration/customs-classification/fasteners/pairwise/`

## Spec Requirements (verbatim)

**Decision 4 (operator 2026-05-08):** Gold-set categories = fasteners + packaging-cartons + examination-gloves verbatim.

**L5-FR-3.1.12 — DoD 12: Gold-set expansion completed for fasteners, packaging-cartons, examination-gloves.**
- AC1: Each category has `canary.inputs.jsonl`, `golden.assertions.yaml`, `metadata.json`, `manifest.md`.
- AC2: Each category passes the §3.4 evidence-tiered accuracy metrics before production use.

**L5-FR-1.2.1 — L5 handles at least three goods categories with decision-grade landed cost.**
- AC1: Fasteners, packaging-cartons, examination-gloves each have calibration fixtures and decision-grade landed-cost outputs.

**L5-FR-3.3.18 — Scenario: gold-set category promotion for the three decided categories.**

## Implementation Details

§11.6 minimum corpus per category:

| Slice | Minimum | Gate |
|-------|---------|------|
| gold | ≥10 | reported separately; auto-safety requires gold participation |
| silver | ≥15 | two-reviewer reasoning per record |
| bronze | ≥10 | training/challenge generation only; cannot certify auto |
| pairwise challenges | ≥12 pairs | share `pair_id`; differ in exactly one material dimension |

Mutation family coverage (bootstrap §7.1 + TDD §9.5):

- `clean`: gold/silver labeled cases with stable evidence URLs
- `weird`: valid hard cases (mixed materials, translation variance, low result count)
- `mutated`: HS6 perturbation, exporter swap, country-of-origin swap, currency swap, packaging weight perturbation, supplementary-unit removal, Incoterm swap, additional-code evidence removal, VAT relief evidence removal, TARIC/HMRC disagreement injection
- `pairwise`: ≥12 pairs sharing `pair_id`, one material evidence dimension changed per pair

Fasteners-specific content:

- Cover HS6 7318 (screws, bolts, nuts) and related: 7318100000 family
- Aluminium structural fasteners (HS 7616) for exporter resolution fixtures (`L5-T-measure_interaction-012` aluminium extrusion fixture overlap is OK)
- Steel structural fasteners for AD/CVD substitutive interaction fixtures
- Exporter resolution: named-exporter vs residual differential
- Supplementary units: kg required (alongside item count)

Format of `manifest.md`:

```markdown
# Fasteners Gold-Set Manifest

## Category Definition
HS Chapter 73 (iron & steel articles), Chapter 76 (aluminium articles), with focus on fasteners (screws, bolts, nuts).

## Trade Remedy Profile
- Steel AD/CVD active (PR review TR0017/TR0018)
- Aluminium extrusions residual rate

## Corpus Stats
- gold: N cases  (each: ATR or import declaration source)
- silver: N cases (each: 2 independent reviewers documented)
- bronze: N cases (tender-doc labels)
- pairwise: N pairs

## Mutation Coverage
- clean: ...
- weird: mixed material variants ...
- mutated: ...
- pairwise: ...
```

`metadata.json`:

```json
{
  "category": "fasteners",
  "version": "1.0.0",
  "operator_decision_date": "2026-05-08",
  "decision_reference": "Decision 4 in docs/strategy/layers/layer-5-expanded-commercial-intelligence.md L5 Pipeline Decisions Recorded section",
  "hs_chapters_in_scope": ["73", "76"],
  "trade_remedy_active": true,
  "minimum_corpus": {"gold": 10, "silver": 15, "bronze": 10, "pairwise_pairs": 12}
}
```

`canary.inputs.jsonl`: each line is a single case JSON with the input descriptor + expected output keys (referenced by `golden.assertions.yaml`).

`golden.assertions.yaml`: keyed by case_id, asserts expected fields per §11.6 metrics.

Tests to add (TDD §8.2):

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-customs_classification-027` | T3 | (covers all 3 categories; this bead contributes fasteners portion) gold-set corpus exists |

Plus this bead contributes coverage for `L5-T-customs_classification-028..032,035` against fasteners-specific data.

## Tests to Pass

- **`L5-T-customs_classification-027`** (T3) — fasteners portion

## Acceptance Criteria

- [ ] 4 root files exist: `canary.inputs.jsonl`, `golden.assertions.yaml`, `metadata.json`, `manifest.md`.
- [ ] 4 mutation subdirectories exist (`clean/`, `weird/`, `mutated/`, `pairwise/`).
- [ ] Minimum corpus counts met (≥10 gold, ≥15 silver, ≥10 bronze, ≥12 pairwise pairs).
- [ ] At least 5 mutation types per `mutated/` (HS6 perturbation + 4 others).
- [ ] Accuracy harness `scoreCategory(fasteners)` returns `production_gate: "eligible"` against this corpus.

## Dependencies

- **Blocked by:** `l5-p6-accuracy-harness-module`.
- **Blocks:** `l5-p6-final-dod-readiness-gate`.
- **Phase:** 6.

---

### l5-p6-goldset-packaging-cartons-corpus

- id: `testproject1-hn05.6.4`
- type: task
- priority: P0
- labels: gold-set,l5,l5-phase-6,layer-5
- parent: `testproject1-hn05.6`
- blocked by: `testproject1-hn05.6.1`

#### Description

# l5-p6-goldset-packaging-cartons-corpus

## SOURCE OF TRUTH WARNING

- **Spec**: §1.2.1, §3.1.12, §3.3.18, §3.6 packaging open question (PRIMARY)
- **TDD**: `L5-T-customs_classification-001` (4819 fixture is packaging-cartons), `027..032,035` (PRIMARY)
- **Strategy**: Phase 6 (after accuracy harness)
- **Bootstrap**: §7.1

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Builds the packaging-cartons gold-set calibration corpus. Cartons are the PACKAGING-EASY category (the validated safe slice per §3.6 packaging open question). Aluminium foil / glass / tin mill / metal cans / composite packaging are DANGEROUS subfamilies requiring broker review — NOT in this corpus; they remain conservative-default until their own gold-set slice exists.

## Files to Create/Modify

Same structure as fasteners corpus:

- **Create:** `tests/fixtures/calibration/customs-classification/packaging-cartons/{canary.inputs.jsonl, golden.assertions.yaml, metadata.json, manifest.md}`
- **Create:** `tests/fixtures/calibration/customs-classification/packaging-cartons/{clean,weird,mutated,pairwise}/`

## Spec Requirements (verbatim)

**Decision 4 (operator 2026-05-08):** Fasteners + packaging-cartons + examination-gloves verbatim.

**L5-FR-3.1.12 — DoD 12.**

**§3.6 open question #9 packaging sub-category reclassification:**
- L5 impact: Determines customs engine treatment of packaging category.
- Current L5 behavior: Packaging-cartons is the validated easy slice; aluminium foil, glass, tin mill, metal cans, and composite packaging are treated as dangerous subfamilies requiring broker/manual review until their own gold-set slice passes.

## Implementation Details

Packaging-cartons-specific content:

- HS 4819 family (cartons, boxes, cases of paper or paperboard) — primary
- HS 4823 (other paper articles) — non-selected alternative branch per §11.3
- HS 3923 (plastic articles for conveyance) — rejected-for-material-mismatch alternative per `L5-T-customs_classification-001`
- Mostly tariff-easy: low duty, no trade remedy, MFN-grade
- VAT treatment: standard rate; no relief unless documented zero/reduced

Mutation focus:

- HS6 perturbation: 4819 ↔ 4823 ↔ 3923 cross-family confusion
- Material perturbation: paperboard vs corrugated vs plastic
- Currency swap: USD/EUR/CNY for cross-border supplier quotes
- Supplementary-unit removal: kg required

`manifest.md` must explicitly EXCLUDE dangerous packaging subfamilies (aluminium foil, glass, tin mill, metal cans, composite) AND document that those subfamilies route to broker until separate gold-set slices exist.

`metadata.json`:

```json
{
  "category": "packaging-cartons",
  "version": "1.0.0",
  "operator_decision_date": "2026-05-08",
  "hs_chapters_in_scope": ["48"],
  "hs_codes_primary": ["4819100000", "4819200000", "4823908580"],
  "explicitly_excluded": ["aluminium_foil_packaging", "glass_packaging", "tin_mill_packaging", "metal_cans", "composite_packaging"],
  "trade_remedy_active": false,
  "minimum_corpus": {"gold": 10, "silver": 15, "bronze": 10, "pairwise_pairs": 12}
}
```

## Tests to Pass

- **`L5-T-customs_classification-027`** (T3) — packaging-cartons portion
- Indirect contribution to `028..032,035`

## Acceptance Criteria

- [ ] 4 root files + 4 mutation subdirectories.
- [ ] Minimum corpus counts met.
- [ ] HS 4819 + 4823 + 3923 represented for the §11.3 tariff graph fixture.
- [ ] Dangerous packaging subfamilies explicitly excluded + documented.
- [ ] Accuracy harness returns `production_gate: "eligible"`.

## Dependencies

- **Blocked by:** `l5-p6-accuracy-harness-module`.
- **Blocks:** `l5-p6-final-dod-readiness-gate`.
- **Phase:** 6.

---

### l5-p6-goldset-examination-gloves-corpus

- id: `testproject1-hn05.6.5`
- type: task
- priority: P0
- labels: gold-set,l5,l5-phase-6,layer-5
- parent: `testproject1-hn05.6`
- blocked by: `testproject1-hn05.6.1`

#### Description

# l5-p6-goldset-examination-gloves-corpus

## SOURCE OF TRUTH WARNING

- **Spec**: §1.2.1, §3.1.12, §3.3.18 (PRIMARY)
- **TDD**: `L5-T-customs_classification-027..032,035` (PRIMARY)
- **Strategy**: Phase 6 (after accuracy harness)
- **Bootstrap**: §7.1

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Builds the examination-gloves gold-set calibration corpus. Gloves are the REGULATED MEDICAL category (medical device classification, sterility/non-sterility, additional codes for medical use). Differentiation focus per operator decision 2026-05-08: "Clear differentiation — remedy-active (fasteners) / packaging-easy (cartons) / regulated medical (examination gloves)."

## Files to Create/Modify

Same structure:

- **Create:** `tests/fixtures/calibration/customs-classification/examination-gloves/{canary.inputs.jsonl, golden.assertions.yaml, metadata.json, manifest.md}`
- **Create:** `tests/fixtures/calibration/customs-classification/examination-gloves/{clean,weird,mutated,pairwise}/`

## Spec Requirements (verbatim)

**Decision 4 (operator 2026-05-08):** fasteners + packaging-cartons + examination-gloves verbatim. Rationale: "Clear differentiation — remedy-active (fasteners) / packaging-easy (cartons) / regulated medical (examination gloves)."

**L5-FR-3.1.12 — DoD 12.**

## Implementation Details

Examination-gloves-specific content:

- HS 4015 (rubber gloves) — primary heading
- HS 6116 (knitted/crocheted gloves) — non-medical alternative
- HS 4203 (leather gloves) — non-medical alternative
- Medical device classification: examination gloves typically Class I medical device under MHRA; impacts VAT relief eligibility
- VAT: VATZ relief possible for medical use evidence; otherwise standard
- Sterility: sterile (e.g., for surgical use) vs non-sterile (examination); different HS subdivisions
- Material: latex / nitrile / vinyl / polyethylene
- Powder vs powder-free
- Compliance evidence: CE mark, MHRA registration, ISO certification

Mutation focus:

- Sterility evidence removal: sterile gloves without sterility evidence → broker required
- Material swap: latex ↔ nitrile ↔ vinyl (different HS subheadings)
- Medical device class swap: Class I ↔ Class II (affects VAT relief)
- VAT relief evidence removal: VATZ claim without medical-use evidence → standard rate
- Origin swap: China → Malaysia → Vietnam → Thailand (affects trade remedy applicability)

This category exercises:

- `L5-FR-1.9.2` VAT relief paths (VATZ / family 6/17)
- `L5-FR-1.6` exporter resolution (Asian manufacturers)
- `L5-FR-1.8.2` supplementary units (pairs required)
- `L5-FR-3.3.10` category whitelist enforcement (medical device must be in validated slice)

`metadata.json`:

```json
{
  "category": "examination-gloves",
  "version": "1.0.0",
  "operator_decision_date": "2026-05-08",
  "hs_chapters_in_scope": ["40", "61", "42"],
  "hs_codes_primary": ["4015110000", "4015190010"],
  "medical_device_classification": "Class I (MHRA)",
  "vat_relief_paths_in_scope": ["VATZ_medical_use"],
  "supplementary_unit_required": "pairs",
  "trade_remedy_active": false,
  "minimum_corpus": {"gold": 10, "silver": 15, "bronze": 10, "pairwise_pairs": 12}
}
```

## Tests to Pass

- **`L5-T-customs_classification-027`** (T3) — examination-gloves portion

Indirect contribution to `028..032,035`.

## Acceptance Criteria

- [ ] 4 root files + 4 mutation subdirectories.
- [ ] Minimum corpus counts met.
- [ ] Sterile vs non-sterile cases included.
- [ ] VAT relief evidence variations represented.
- [ ] Material variants (latex/nitrile/vinyl) all covered.
- [ ] Accuracy harness returns `production_gate: "eligible"`.

## Dependencies

- **Blocked by:** `l5-p6-accuracy-harness-module`.
- **Blocks:** `l5-p6-final-dod-readiness-gate`.
- **Phase:** 6.

---

### l5-p6-cbam-placeholder

- id: `testproject1-hn05.6.6`
- type: task
- priority: P1
- labels: cbam,l5,l5-phase-6,layer-5
- parent: `testproject1-hn05.6`
- blocked by: `testproject1-hn05.4.13`

#### Description

# l5-p6-cbam-placeholder

## SOURCE OF TRUTH WARNING

- **Spec**: §1.14.1, §1.14.2, §1.14.3, §3.3.19, §3.7.8 (PRIMARY)
- **TDD**: `L5-T-customs_classification-025,026` (PRIMARY)
- **Strategy**: Phase 6 (CBAM exposure flag only — rate-bearing field stays null)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

CBAM (UK Carbon Border Adjustment Mechanism) — forward placeholder logic. Per §3.6 #10 open question, UK CBAM trial rates expected Q4 2026, full rates January 2027. L5 emits ONLY exposure flags for confirmed 2027 sectors (aluminium, cement, fertiliser, hydrogen, iron and steel) with delivery windows crossing 2027-01-01. `cbam_estimate_gbp_pence` REMAINS NULL until rate rules + calculation method are final.

## Files to Create/Modify

- **Modify:** `packages/intelligence/src/customs/classification-freshness.ts` OR a dedicated CBAM module within an existing customs file (don't add a 7th customs file — package inventory is fixed at 6 customs files).
- **Modify:** `packages/intelligence/src/customs/customs-classification.test.ts` (add `L5-T-customs_classification-025,026`)

## Spec Requirements (verbatim)

**L5-FR-1.14.1 — CBAM remains a forward placeholder until UK rate rules are final.**
- AC1: `cbamEstimateGbp`/`cbam_estimate_gbp_pence` remains null until secondary legislation finalises rates and calculation method.
- AC2: Confirmed 2027 sector scope is aluminium, cement, fertiliser, hydrogen, iron and steel; glass and ceramics are not in 2027 scope.

**L5-FR-1.14.2 — Tender windows crossing 2027-01-01 surface future regulatory exposure.**
- AC1: Tenders with delivery windows crossing 2027-01-01 and affected commodity families set `cbam_exposure_flag = 1` even when the estimate remains null.
- AC2: The flag is an exposure warning, not a quantified cost.

**L5-FR-1.14.3 — CBAM exposure flag has explicit clear conditions.**
- AC1: `cbam_exposure_flag` is left `0` or null when the commodity family is outside confirmed 2027 scope.
- AC2: `cbam_exposure_flag` is left `0` or null when the expected import or delivery window ends before 2027-01-01.
- AC3: `cbam_estimate_gbp_pence` remains null while exposure flagging can still be active.

**L5-FR-3.3.19 — Scenario: CBAM exposure flag for a 2027-crossing tender window.**
- AC1: An in-scope aluminium or iron/steel family with delivery crossing 2027-01-01 sets `cbam_exposure_flag = 1` and keeps `cbam_estimate_gbp_pence = null`.

**L5-FR-3.7.8 — Risk: CBAM rate publication delay.**

## Implementation Details

```ts
const CBAM_2027_SECTOR_HS_PREFIXES = [
  "72", "73",          // iron and steel
  "76",                // aluminium
  "25", "23", "31",    // cement, fertilisers
  "28",                // hydrogen, ammonia, urea
];

const CBAM_START_DATE = "2027-01-01";

export interface CbamFlagInput {
  uk10: string;                   // e.g., "7318159590"
  deliveryWindow: { start: string; end: string };
}

export interface CbamFlagOutput {
  cbamExposureFlag: 0 | 1;
  cbamEstimateGbpPence: null;     // ALWAYS null in L5
  rationale: string;
}

export function evaluateCbamExposure(input: CbamFlagInput): CbamFlagOutput {
  const sectorMatch = CBAM_2027_SECTOR_HS_PREFIXES.some(p => input.uk10.startsWith(p));
  const crossesCutover = input.deliveryWindow.end >= CBAM_START_DATE;
  const flag = sectorMatch && crossesCutover ? 1 : 0;
  return {
    cbamExposureFlag: flag,
    cbamEstimateGbpPence: null,
    rationale: flag ? "in-scope 2027 sector with delivery crossing 2027-01-01" : "outside CBAM scope or delivery before 2027-01-01"
  };
}
```

Glass (chapter 70) and ceramics (chapter 69) are NOT in 2027 scope per §1.14.1 AC2. Confirmed sectors: iron & steel (chapters 72-73), aluminium (76), cement (chapter 25.23), fertilisers (chapter 31), hydrogen (within chapter 28).

Tests to add:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-customs_classification-025` | T1 | aluminium, cement, fertiliser, hydrogen, iron/steel with delivery windows crossing 2027-01-01 → `cbam_exposure_flag = 1`, `cbam_estimate_gbp_pence = null` |
| `L5-T-customs_classification-026` | T1 | glass + ceramics + any delivery ending before 2027-01-01 → `cbam_exposure_flag` false/null, `cbam_estimate_gbp_pence = null` |

## Tests to Pass

- **`L5-T-customs_classification-025`** (T1)
- **`L5-T-customs_classification-026`** (T1)

## Acceptance Criteria

- [ ] `evaluateCbamExposure` function exported.
- [ ] `cbam_estimate_gbp_pence` is ALWAYS null in L5 (rate-bearing logic forbidden).
- [ ] 5 in-scope 2027 sectors (aluminium, cement, fertiliser, hydrogen, iron/steel) covered.
- [ ] Glass + ceramics not in confirmed 2027 CBAM sector list per §1.14.1 AC2.
- [ ] `landed_cost_estimates.cbam_exposure_flag` and `cbam_estimate_gbp_pence` populated via existing columns.
- [ ] Both test IDs pass.

## Dependencies

- **Blocked by:** `l5-p4-customs-scope-preservation-gate`.
- **Blocks:** `l5-p6-final-dod-readiness-gate`.
- **Phase:** 6.

---

### l5-p6-field-level-projection-boundary

- id: `testproject1-hn05.6.7`
- type: task
- priority: P0
- labels: l5,l5-phase-6,layer-5,redaction
- parent: `testproject1-hn05.6`
- blocked by: `testproject1-hn05.4.13`, `testproject1-hn05.4.12`

#### Description

# l5-p6-field-level-projection-boundary

## SOURCE OF TRUTH WARNING

- **Spec**: §1.3.1, §3.8.1 (PRIMARY)
- **TDD**: `L5-T-measure_interaction-020`, §11.8 (Redaction Boundary Contract) (PRIMARY)
- **Strategy**: Phase 6 (final projection boundary implementation; design done in Phase 4)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Phase 4 produced the redaction sentinel taxonomy. This bead implements the actual field-level projection boundary so the viewer role cannot read automation-sensitive sentinels, while the operator role can read all decision-grade fields. Strategy: existing L4 table-level RLS grants whole-table SELECT, so this bead must add a redacted view, role-scoped RPC serializer, or equivalent projection path.

## Files to Create/Modify

- **Create or modify:** `infra/supabase/migrations/NNNN_l5_field_level_projection.sql` (next-free Supabase migration adding a redacted view OR new RLS policies that filter columns by role)
- **Possibly modify:** `crates/daemon/src/db/sync.rs` (push set) — if viewer projection requires a separate push table per role; preferred is a Postgres-side redacted view
- **Modify:** `packages/intelligence/src/customs/measure-interaction.test.ts` (close `L5-T-measure_interaction-020`)

## Spec Requirements (verbatim)

**L5-FR-1.3.1 — L5 populates existing `landed_cost_estimates` columns without adding columns to that table.**
**L5-FR-3.8.1 AC2:** Quote-role quality gate blocks opportunities backed only by `domestic_wholesale_floor` or `optimistic_floor`.

§11.8 Redaction Boundary Contract (verbatim — 4 classifications):

| Class | Fields | Viewer | Operator |
|-------|--------|--------|----------|
| display | `declarable_commodity_code_10`, `net_margin_low/base/high_bps`, `decision_grade`, `warnings`, `blockers`, `cbam_exposure_flag` | present | present |
| control | `customs_fx_rate`, `commercial_fx_rate`, `fx_basis_delta_bps`, `freight_low/base/high_gbp_pence`, `customs_duty_gbp_pence`, `trade_remedy_duty_gbp_pence` | present only as rounded/decision-grade; raw artifacts absent | present |
| automation-sensitive | `exporter_rate_basis`, `exporter_identity_confidence_bps`, `origin_evidence_state`, `trade_remedy_cases_json`, `duty_breakdown_json`, `measure_interaction_json`, `confidence_breakdown.evidence_leverage` | absent by field name AND sentinel value | present |
| local-only sensitive | browser transcripts, raw marketplace pages, skill checkpoints, service keys, credential state | absent | absent from shared projection; local desktop artifacts only |

## Implementation Details

Recommended approach: Postgres-side redacted view + role-based grants.

```sql
-- NNNN_l5_field_level_projection.sql

-- Viewer-redacted view: drops automation-sensitive columns
CREATE VIEW landed_cost_estimates_viewer AS
SELECT
  id, process_id, notice_id, product_candidate_id, supplier_quote_id,
  -- display fields (full visibility)
  declarable_commodity_code_10,
  net_margin_low_bps, net_margin_base_bps, net_margin_high_bps,
  decision_grade, warnings, blockers, cbam_exposure_flag,
  -- control fields (rounded; raw artifacts absent — viewer reads same DB row, doesn't see local-only artifacts)
  customs_fx_rate, commercial_fx_rate, fx_basis_delta_bps,
  freight_low_gbp_pence, freight_base_gbp_pence, freight_high_gbp_pence,
  customs_duty_gbp_pence, trade_remedy_duty_gbp_pence,
  -- automation-sensitive fields OMITTED:
  --   exporter_rate_basis, exporter_identity_confidence_bps, origin_evidence_state,
  --   trade_remedy_cases_json, duty_breakdown_json, measure_interaction_json
  -- L4 sync fields preserved
  owner_id, row_version, last_local_seq, stage_run_id, updated_at, created_at
FROM landed_cost_estimates;

-- Strip evidence_leverage from confidence_breakdown for viewer
-- (confidence_breakdown stays in operator projection only OR strip via a JSON function)

-- Grants: viewer role can SELECT the view; operator role can SELECT the base table
REVOKE ALL ON TABLE landed_cost_estimates FROM viewer_role;
GRANT SELECT ON landed_cost_estimates_viewer TO viewer_role;
GRANT SELECT ON landed_cost_estimates TO operator_role;
```

Approach for `product_commodity_codes` and any L5 push-owned tables: same pattern — redacted view for viewer, base table for operator.

Critical invariant: existing L4 RLS grants whole-table SELECT to viewer. This bead must REVOKE that direct grant on the L5 sensitive columns AND substitute the redacted view. Test `L5-T-measure_interaction-020` seeds sentinel values into automation-sensitive fields and asserts: viewer-scoped Supabase session CANNOT see those sentinel values AND the field names absent from response JSON.

Implementation closes the `L5-T-measure_interaction-020` test:

| Test ID | Tier | Assertion |
|---------|------|-----------|
| `L5-T-measure_interaction-020` | T2 | viewer-role projection reads display + rounded control fields; cannot read automation-sensitive sentinels (by field name AND sentinel value); operator reads operator/control fields; both roles read-only outside approved RPCs |

## Tests to Pass

- **`L5-T-measure_interaction-020`** (T2) — full closure

## Acceptance Criteria

- [ ] Field-level projection boundary implemented (redacted view, role grants, OR equivalent RPC serializer).
- [ ] Viewer cannot see any of 7 automation-sensitive fields by field name OR sentinel value.
- [ ] Operator can see all decision-grade fields.
- [ ] Neither role can WRITE to push-owned rows outside approved L4 RPC paths.
- [ ] `L5-T-measure_interaction-020` passes with sentinel-value assertion.
- [ ] No new column added to `landed_cost_estimates`.

## Dependencies

- **Blocked by:** `l5-p4-redaction-sentinel-design`, `l5-p4-customs-scope-preservation-gate`.
- **Blocks:** `l5-p6-mainrs-delegation-cutover`, `l5-p6-final-dod-readiness-gate`.
- **Phase:** 6.

---

### l5-p6-mainrs-feature-flag-shadow

- id: `testproject1-hn05.6.8`
- type: task
- priority: P0
- labels: l5,l5-phase-6,layer-5,mainrs
- parent: `testproject1-hn05.6`
- blocked by: `testproject1-hn05.5.9`, `testproject1-hn05.4.13`, `testproject1-hn05.3.7`, `testproject1-hn05.2.6`

#### Description

# l5-p6-mainrs-feature-flag-shadow

## SOURCE OF TRUTH WARNING

- **Spec**: §2.4.1, §3.1.11 (PRIMARY)
- **TDD**: integration-test surface; cited test IDs across customs/FX/freight/marketplace (PRIMARY)
- **Strategy**: Phase 6 (3-bead `main.rs` cutover: this bead = feature flag + shadow)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

The `main.rs` landed-cost path is the largest single bead surface in L5: 208 textual `landed_cost` touch points across lines 5639-8288 (2650-line implementation span). Strategy mandates a 3-bead split: (1) feature flag + shadow, (2) delegation cutover, (3) default-on + live demo. This is bead (1): default-OFF decision-grade feature flag + shadow comparison against the existing screening-grade path.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/main.rs` (introduce feature flag + shadow-compare invocation; do not delete old path)
- **Possibly create:** `crates/daemon/src/main_landed_cost_decision_grade.rs` module to host the new path cleanly (extracted from main.rs)
- **Possibly extend:** `crates/daemon/tests/customs_classification.rs` with shadow-comparison test

## Spec Requirements (verbatim)

**L5-FR-2.4.1 — L5 populates every `landed_cost_estimates` column or preserves its L4 sync semantics.**
**L5-FR-3.1.11 — DoD 11: decision-grade landed cost per validated category with full audit trail.**

From strategy §Phase 6 ordering: "Stage 11 live-loop integration first runs old and new paths side-by-side under a default-OFF decision-grade feature flag, then delegates to package engines by default only after comparison, Contracts Finder live demo, and regression gates pass."

## Implementation Details

Feature flag introduction:

```rust
// in main.rs or a new module
pub const L5_DECISION_GRADE_LANDED_COST_FEATURE_FLAG_ENV: &str = "L5_ENABLE_DECISION_GRADE_LANDED_COST";

pub enum LandedCostPathMode {
    ScreeningGradeOnly,        // default; existing path
    ShadowCompare,              // both paths run; results compared, screening-grade returned to caller
    DecisionGrade,              // new path returns; old path is dead code
}

fn read_landed_cost_mode_from_env() -> LandedCostPathMode {
    match std::env::var(L5_DECISION_GRADE_LANDED_COST_FEATURE_FLAG_ENV).as_deref() {
        Ok("shadow") => LandedCostPathMode::ShadowCompare,
        Ok("decision_grade") => LandedCostPathMode::DecisionGrade,
        _ => LandedCostPathMode::ScreeningGradeOnly,
    }
}
```

Shadow comparison: both paths run on the same inputs; compare outputs field-by-field (`customs_duty_gbp_pence`, `freight_base_gbp_pence`, `decision_grade`, etc.). Divergence persisted to a `landed_cost_path_comparison` table (LOCAL-ONLY, never pushed) for operator review.

This bead introduces the flag + shadow comparison. It does NOT delete the old path or default-on the new path.

Tests to add (extend `customs_classification.rs` daemon target):

```rust
#[test]
fn l5_decision_grade_feature_flag_defaults_to_screening() {
    std::env::remove_var(L5_DECISION_GRADE_LANDED_COST_FEATURE_FLAG_ENV);
    let mode = read_landed_cost_mode_from_env();
    assert!(matches!(mode, LandedCostPathMode::ScreeningGradeOnly));
}

#[test]
fn l5_shadow_compare_records_divergence() {
    std::env::set_var(L5_DECISION_GRADE_LANDED_COST_FEATURE_FLAG_ENV, "shadow");
    // Run both paths against a known fixture, assert divergence row persisted
}
```

## Tests to Pass

- Daemon-level smoke that feature flag defaults to screening-grade (no behavior change without env).
- Shadow comparison produces divergence records on at least 1 fixture per gold-set category.

## Acceptance Criteria

- [ ] Feature flag env var defined; default = `ScreeningGradeOnly`.
- [ ] Shadow mode runs both paths and persists divergence locally.
- [ ] New path does NOT become authoritative without explicit env var.
- [ ] Old path REMAINS functional (no deletion in this bead).
- [ ] `cargo test -p daemon --bin daemon` passes.
- [ ] No regressions in existing landed-cost path callers.

## Dependencies

- **Blocked by:** `l5-p2-fx-cron-persistence-gate`, `l5-p3-freight-source-health-gate`, `l5-p4-customs-scope-preservation-gate`, `l5-p5-skills-scope-preservation-gate`.
- **Blocks:** `l5-p6-mainrs-delegation-cutover`.
- **Phase:** 6.

---

### l5-p6-mainrs-delegation-cutover

- id: `testproject1-hn05.6.9`
- type: task
- priority: P0
- labels: l5,l5-phase-6,layer-5,mainrs
- parent: `testproject1-hn05.6`
- blocked by: `testproject1-hn05.6.8`, `testproject1-hn05.6.7`

#### Description

# l5-p6-mainrs-delegation-cutover

## SOURCE OF TRUTH WARNING

- **Spec**: §2.4.1, §3.1.11, §3.8.1, §3.8.2 (PRIMARY)
- **TDD**: integration-test surface (PRIMARY)
- **Strategy**: Phase 6 (3-bead split: this is bead 2 — delegation)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Delegates the live `main.rs` Stage 11 landed-cost path to the new package engines (`packages/intelligence/src/{fx,logistics,customs}/`). Once delegation lands, the new path produces `landed_cost_estimates` rows via the package-side `FxEngine`, `FreightEngine`, and customs orchestrator. Feature flag still defaults OFF; this bead WIRES the delegation under the flag so `decision_grade` mode actually runs the new engines.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/main.rs` (Stage 11 landed-cost path delegates to package engines under `DecisionGrade` mode)
- **Possibly create:** `crates/daemon/src/landed_cost_decision_grade_bridge.rs` (Rust ↔ TypeScript shim — actually this is unlikely; if the package engines are TS-only, the Rust side must either run the TS via subprocess OR the entire decision-grade pipeline lives in TypeScript with results piped back to Rust. The codebase architecture has `packages/intelligence/` as a Bun/TS workspace consumed by the daemon via some integration boundary — read existing usage patterns to determine the correct shim style.)

## Spec Requirements (verbatim)

**L5-FR-2.4.1** + **L5-FR-3.1.11** + **L5-FR-3.8.1** (L6 readiness producers persisted).

## Implementation Details

Cutover order (within this bead, internal sub-steps):

1. Read existing `crates/daemon/src/main.rs:5639-8288` Stage 11 landed-cost block.
2. Identify the integration boundary the daemon uses to call into `packages/intelligence/` (existing files: `crates/daemon/tests/agent_supplier_enrichment.rs` may show the pattern; or there's a subprocess/IPC pathway).
3. Behind the feature flag `LandedCostPathMode::DecisionGrade`, replace the landed-cost arithmetic with calls into:
   - `FxEngine` (Phase 2)
   - `FreightEngine` (Phase 3)
   - `multi-source-recall` + `ten-digit-resolution` + `measure-scanner` + `exporter-resolution` + `classification-freshness` + `accuracy` (Phase 4 + 6)
4. Populate all `landed_cost_estimates` columns from §2.4 verbatim mapping table; no schema change.
5. Emit all L5 error codes via the daemon enum (Phase 1 work).
6. Persist `product_commodity_codes` rows via Phase 4 `persistRecallLattice`.
7. Apply the field-level projection boundary at Supabase push time (Phase 6 `l5-p6-field-level-projection-boundary` provides the redacted view).

Shadow mode comparison (from prior bead) STILL active in this bead's tests — but here `DecisionGrade` mode actually replaces the path output, not just shadow-runs alongside.

Old path: NOT deleted. Kept as `ScreeningGradeOnly` mode for emergency fallback. Will be deleted only after default-on bead proves stability.

## Tests to Pass

- Integration test: setting `L5_ENABLE_DECISION_GRADE_LANDED_COST=decision_grade` causes the new path to produce the `landed_cost_estimates` row.
- All Phase 4 customs test IDs continue passing under decision_grade mode (the new path is the same engines, so already proven).
- No regressions in `cargo test -p daemon --bin daemon`.

## Acceptance Criteria

- [ ] `DecisionGrade` mode delegates to `packages/intelligence/` engines.
- [ ] All `landed_cost_estimates` columns populated per §2.4 mapping.
- [ ] No new `landed_cost_estimates` columns.
- [ ] Field-level projection boundary applied at push time.
- [ ] Feature flag still default-OFF — production is unaffected.
- [ ] `cargo test -p daemon --bin daemon` + daemon integration target loop passes.
- [ ] Old path code preserved.

## Dependencies

- **Blocked by:** `l5-p6-mainrs-feature-flag-shadow`, `l5-p6-field-level-projection-boundary`.
- **Blocks:** `l5-p6-mainrs-default-on-and-live-demo`.
- **Phase:** 6.

---

### l5-p6-mainrs-default-on-and-live-demo

- id: `testproject1-hn05.6.10`
- type: task
- priority: P0
- labels: l5,l5-phase-6,layer-5,live-demo,mainrs
- parent: `testproject1-hn05.6`
- blocked by: `testproject1-hn05.6.9`, `testproject1-hn05.6.5`, `testproject1-hn05.6.4`, `testproject1-hn05.6.3`

#### Description

# l5-p6-mainrs-default-on-and-live-demo

## SOURCE OF TRUTH WARNING

- **Spec**: §2.4.1, §3.1.11 (PRIMARY)
- **TDD**: §3 Stage Validation Test Gate, §11 fixture contracts (PRIMARY)
- **Strategy**: Phase 6 (3-bead split: this is bead 3 — default-on + Contracts Finder live demo)

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

After delegation cutover passes regression gates AND shadow comparison shows no material divergence, default the feature flag ON. Run live demo against a real Contracts Finder lot through the entire L5 pipeline to confirm decision-grade landed-cost output end-to-end.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/main.rs` (change default of `read_landed_cost_mode_from_env` to `DecisionGrade`)
- **Create:** `docs/pipeline/l5-expanded-commercial-intelligence/live-demo/contracts-finder-lot-evidence.md` (live demo record)

## Spec Requirements (verbatim)

**L5-FR-3.1.11 — DoD 11: Decision-grade landed cost per validated category with full audit trail.**

Strategy §Phase 6: "Stage 11 live-loop integration first runs old and new paths side-by-side under a default-OFF decision-grade feature flag, then delegates to package engines by default only after comparison, Contracts Finder live demo, and regression gates pass."

## Implementation Details

Default change:

```rust
fn read_landed_cost_mode_from_env() -> LandedCostPathMode {
    match std::env::var(L5_DECISION_GRADE_LANDED_COST_FEATURE_FLAG_ENV).as_deref() {
        Ok("screening_grade") | Ok("off") => LandedCostPathMode::ScreeningGradeOnly,
        Ok("shadow") => LandedCostPathMode::ShadowCompare,
        // changed default:
        _ => LandedCostPathMode::DecisionGrade,
    }
}
```

This bead's pre-cutover gate (must all be green before changing default):

1. Shadow comparison run for ≥7 days; divergence < 5% in any field on >80% of fixtures.
2. All Phase 4 customs test IDs pass under `decision_grade` mode.
3. All Phase 2 FX test IDs pass under `decision_grade` mode.
4. All Phase 3 freight test IDs pass under `decision_grade` mode.
5. All gold-set fixtures (Phase 6.3/6.4/6.5) score `production_gate: eligible`.
6. Field-level projection boundary verified by `L5-T-measure_interaction-020`.
7. L5→L6 readiness fixture proves every blocker condition can be produced.

Live demo procedure (recorded in `docs/pipeline/l5-expanded-commercial-intelligence/live-demo/contracts-finder-lot-evidence.md`):

1. Select a recent Contracts Finder lot in one of the 3 gold-set categories.
2. Run the full L5 pipeline (procurement-ingestion → product-extraction → supplier search → customs classification → FX → freight → decision-grade landed cost).
3. Capture: lot ID, ingested record, product candidate, recall lattice trace, supplier quotes from ≥2 sources, full `landed_cost_estimates` row, decision_grade verdict.
4. Document: any unexpected blockers, FX basis delta, freight provider disagreement, classification freshness state, evidence leverage entries.
5. Operator sign-off recorded in the demo doc.

Old screening-grade path: AFTER default-on demonstrates stability for some period (post-L5), the old path can be deleted. NOT deleted by this bead — preserved as opt-in via `L5_ENABLE_DECISION_GRADE_LANDED_COST=screening_grade` for emergency rollback.

## Tests to Pass

- Stage Validation Test Gate full run (Section 3 / §3.2.2): `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test -p daemon --bin daemon && for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done && bun run build && bun run test`
- All L5 package + daemon integration target test IDs (all 98 L5-T IDs + 8 CI gates).

## Acceptance Criteria

- [ ] Feature flag default flipped to `DecisionGrade`.
- [ ] Pre-cutover gate 1-7 all green.
- [ ] Contracts Finder live demo executed + recorded with operator sign-off.
- [ ] Stage Validation Test Gate passes 100%.
- [ ] Emergency rollback path preserved (set env var to `screening_grade`).

## Dependencies

- **Blocked by:** `l5-p6-mainrs-delegation-cutover`, all gold-set corpus beads.
- **Blocks:** `l5-p6-final-dod-readiness-gate`.
- **Phase:** 6.

---

### l5-p6-final-dod-readiness-gate

- id: `testproject1-hn05.6.11`
- type: task
- priority: P0
- labels: dod,l5,l5-phase-6,layer-5,review-gate
- parent: `testproject1-hn05.6`
- blocked by: `testproject1-hn05.6.10`, `testproject1-hn05.6.9`, `testproject1-hn05.6.8`, `testproject1-hn05.6.7`, `testproject1-hn05.6.5`, `testproject1-hn05.6.4`, `testproject1-hn05.6.3`, `testproject1-hn05.6.1`, `testproject1-hn05.6.6`, `testproject1-hn05.6.2`

#### Description

# l5-p6-final-dod-readiness-gate

## SOURCE OF TRUTH WARNING

- **Spec**: §3.1 (DoD 1-12), §3.2 (test files inventory), §3.8 (L5→L6 readiness gate) (PRIMARY)
- **TDD**: ALL L5-T-* IDs (98), CI-L5-001..008 (PRIMARY)
- **Strategy**: Phase 6 final gate; L5 exit criteria

## Coordination Protocol

`bin/br-claim <bead-id>` / `bin/br-complete <bead-id>`.

## Context

Final L5 exit gate. Verifies the full Stage Validation Test Gate passes, ALL 12 DoD rows pass in one walkthrough, ALL 98 L5-T-* test IDs pass, ALL 8 CI-L5-* gates pass, ALL 19 package files present, AND the L5→L6 readiness fixture proves every blocker condition produces and blocks bid readiness.

## Files to Inspect

ALL Phase 1-6 bead outputs.

## Gate Criteria (from strategy + §3.1 + §3.8)

DoD walkthrough (§3.1 — must walk through all 12 in one pass):

1. **DoD 1** (`L5-T-freight_engine-009,010,013`): Alibaba/1688 operational with Chinese keyword clusters.
2. **DoD 2** (`L5-T-freight_engine-011,013`): Amazon operational (browser-first).
3. **DoD 3** (`L5-T-freight_engine-014`): PCS Scotland ingestion operational.
4. **DoD 4** (`L5-T-hmrc_fpo-002,003,008`): HMRC FPO/search-suggestion endpoint investigation completed.
5. **DoD 5** (`L5-T-hmrc_fpo-004,005,010`): Multi-source recall produces from ≥2 independent sources.
6. **DoD 6** (`L5-T-customs_classification-001,002,004`): 10-digit resolution via HMRC.
7. **DoD 7** (`L5-T-measure_interaction-001..006`): Measure scanner detects AD/CVD/safeguard/quota and models interactions.
8. **DoD 8** (`L5-T-fx_engine-001,005,009`): HMRC customs FX.
9. **DoD 9** (`L5-T-fx_engine-002,003,011`): Frankfurter spot FX.
10. **DoD 10** (`L5-T-freight_engine-003,004,005`): Freightos + SeaRates.
11. **DoD 11** (`L5-T-customs_classification-009,012,L5-T-fx_engine-010`): Decision-grade landed cost with full audit trail.
12. **DoD 12** (`L5-T-customs_classification-027..032`): Gold-set expansion for 3 categories.

L5→L6 readiness fixture (§3.8.1) — must produce each blocker:

- `CLASSIFICATION_AMBIGUOUS`
- `TARIFF_BRANCH_DIVERGENCE`
- required-component `SUPPLEMENTARY_UNIT_MISSING`
- `decision_grade = broker_required`
- expired `classification_fresh_until`
- floor-only quote-role gate

CI gates (§7):

- `CI-L5-001`..`CI-L5-008` all pass.

## Implementation Details

Verification script (this bead executes):

```bash
# 1. Stage Validation Test Gate (mandatory, complete)
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
bun run build
bun run test

# 2. Spec FR coverage — every L5-FR maps to at least one bead description or AC
SPEC_IDS=$(rg -o 'L5-FR-[0-9]+\.[0-9]+\.[0-9]+' docs/strategy/layers/layer-5-expanded-commercial-intelligence.md | sort -u)
BEAD_DESCS=$(br list --json | jq -r '.issues[].description')
MISSING=0
for id in $SPEC_IDS; do
  if ! echo "$BEAD_DESCS" | grep -q "$id"; then
    echo "MISSING: $id"
    MISSING=$((MISSING+1))
  fi
done
test "$MISSING" -eq 0 || { echo "[FAIL] $MISSING spec FRs missing from beads"; exit 1; }

# 3. Test ID coverage — every L5-T owned by exactly one bead
TEST_IDS=$(rg -o 'L5-T-[a-z0-9_]+-[0-9]+' docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md | sort -u)
for id in $TEST_IDS; do
  COUNT=$(echo "$BEAD_DESCS" | grep -c "$id" || true)
  if [ "$COUNT" -lt 1 ]; then
    echo "UNASSIGNED: $id"
    exit 1
  fi
done

# 4. 19 package files
test 6 -eq $(ls packages/intelligence/src/customs/*.ts | grep -v ".test.ts" | wc -l)
test 4 -eq $(ls packages/intelligence/src/fx/*.ts | grep -v ".test.ts" | wc -l)
test 5 -eq $(ls packages/intelligence/src/logistics/*.ts | grep -v ".test.ts" | wc -l)
test 3 -eq $(ls packages/intelligence/src/validation/*.ts | wc -l)
test 1 -eq $(ls packages/intelligence/src/customs/accuracy.ts | wc -l)

# 5. 5 package test files
test -f packages/intelligence/src/customs/hmrc-fpo.test.ts
test -f packages/intelligence/src/customs/customs-classification.test.ts
test -f packages/intelligence/src/customs/measure-interaction.test.ts
test -f packages/intelligence/src/fx/fx-engine.test.ts
test -f packages/intelligence/src/logistics/freight-engine.test.ts

# 6. 3 daemon integration test files
test -f crates/daemon/tests/customs_classification.rs
test -f crates/daemon/tests/customs_fx_freight_engine.rs
test -f crates/daemon/tests/customs_classification_accuracy.rs

# 7. 3 gold-set corpora
for cat in fasteners packaging-cartons examination-gloves; do
  for f in canary.inputs.jsonl golden.assertions.yaml metadata.json manifest.md; do
    test -f "tests/fixtures/calibration/customs-classification/$cat/$f" || exit 1
  done
done

# 8. L5→L6 readiness fixture — produce each blocker
bun test packages/intelligence/src/customs/customs-classification.test.ts -t "L5-T-customs_classification-034"
# This test asserts L6 cannot reach ready_to_bid while any L5 blocker, broker-required grade, expired freshness, or floor-only quote role is active

# 9. DoD walkthrough — single pass over all 12 DoD test ID sets
# (each DoD row's test IDs are exercised in one run by the package test suite + daemon integration loop)
```

## Tests to Pass

- All 98 L5-T-* test IDs.
- All 8 CI-L5-* gates.
- All 12 DoD rows.

## Acceptance Criteria

- [ ] Stage Validation Test Gate passes 100%.
- [ ] All 130 L5-FR-* IDs covered by at least one bead description.
- [ ] All 98 L5-T-* test IDs assigned to exactly one bead.
- [ ] All 19 package files present (6/4/5/3/1 distribution).
- [ ] All 5 package test files + 3 daemon integration files present.
- [ ] All 3 gold-set corpora pass evidence-tiered metrics.
- [ ] L5→L6 readiness gate produces all 5 blocker conditions + floor-only quote-role gate.
- [ ] DoD walkthrough document recording all 12 DoD rows passing in one pass.
- [ ] All Phase 1-6 task beads closed via `bin/br-complete`.

## Dependencies

- **Blocked by:** ALL prior Phase 1-6 task beads + phase review gates.
- **Blocks:** L6 epic (any future L6 work cannot start until this bead closes).
- **Phase:** 6 (FINAL).

---

