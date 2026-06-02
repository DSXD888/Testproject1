# Goods-Led Install/Maintenance + UK Labour-Cost Engine - TDD Specification

> Document Type: Source of Truth #2 (TDD)
> Feature: Goods-Led Supply-and-Install / Maintenance Expansion + UK Labour-Cost Engine
> Created: 2026-06-02
> Spec: `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md`
> Bootstrap: `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-bootstrap.md`

This document defines the tests that must exist before the feature can be declared implemented. The bootstrap document defines how those tests are implemented and run.

## 1. Scope

Primary inputs:

- `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md`
- `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-architecture.md`
- `docs/research/2026-06-01-goods-led-install-maintenance-pipeline-research.md` section 6.3, section 6.4, Appendix A, Appendix E, and Appendix F.
- Existing test harnesses:
  - `crates/daemon/tests/customs_fx_freight_engine.rs`
  - `crates/daemon/tests/decision_basis_hash.rs`
  - `crates/daemon/tests/skill_canary_harness.rs`
  - `crates/daemon/tests/skill_lint.rs`
  - `packages/intelligence/src/qualification/goods-qualification.test.ts`
  - `packages/intelligence/src/margins/coverage-rollup.test.ts`
  - `packages/intelligence/src/customs/customs-classification.test.ts`
  - `tests/fixtures/calibration/customs-classification/`

Spec inventory:

- 35 functional requirement IDs.
- 32 Tier-A committed requirements: `FR-Q.*`, `FR-L.*`, `FR-R.*`, `FR-U.*`, `FR-V.*`.
- 3 Tier-B bounded research-spike requirements: `FR-Z.1`, `FR-Z.2`, `FR-Z.3`.
- 182 acceptance-criteria labels in the spec.

## 2. Non-Negotiable Test Principles

1. Real harness only: no mocks, no HTTP stubs, no fake SQLite state, no existence-only assertions.
2. Every Tier-A requirement has at least one named test ID in the coverage matrix.
3. Every Tier-B requirement appears in the coverage matrix as research-spike coverage only. Do not invent code assertions for `FR-Z.*`.
4. `TS-QUAL-003` must be split into `TS-QUAL-003a` and `TS-QUAL-003b`. The old 40 percent `pass` expectation is invalid.
5. `description-quality.test.ts` is a new hard prerequisite and must contain the full eight-row service-gate truth table.
6. Legacy direct `landed_cost_estimates` INSERT tests must keep passing through nullable/default-0 schema additions.
7. The Scenario 1 NW labour band is a hard engine-acceptance assertion: `210000 <= installation_labour_cost_gbp_pence <= 265000`.
8. The green bar is the full AGENTS.md mixed-stack gate, including every `crates/daemon/tests/*.rs` integration target and `bun test`.

## 3. Test Pyramid

| Tier | Purpose | Primary surfaces | Command |
|---|---|---|---|
| T1 TypeScript unit | Qualification branch, service-gate split, goods-share arithmetic, coverage rollup metrics | `packages/intelligence/src/**/*.test.ts` | `bun test ./src` from `packages/intelligence` or root `bun test` |
| T1 Rust unit | Domestic/import rollup math, field persistence helpers, saturating pence arithmetic | daemon bin tests in touched modules | `cargo test -p daemon --bin daemon` |
| T2 Rust integration | Real SQLite seeding, schema epoch, deterministic Appendix-E scenarios, decision basis hashing | `crates/daemon/tests/*.rs` | `cargo test -p daemon --test installation_labour_engine` plus full integration loop |
| T2 Skill lint/canary | Labour worker metadata, fixture corpus shape, warning-code coverage, overlay canary | `skill_lint.rs`, `skill_canary_harness.rs` | `cargo test -p daemon --test skill_lint`; `cargo test -p daemon --test skill_canary_harness` |
| T3 UI E2E | Manifest viewer and contract-card rendering of labour/permit breakdown | `/agent-browser` flow against desktop/web | agent-browser flow in Appendix F.3 |
| T4 Live smoke | `run_landed_cost_live_loop` transient-loads the labour skill with no browser dependency | seeded live landed-cost loop | nightly smoke target |

## 4. CI And Local Gate Inventory

| Gate ID | Gate | Proves |
|---|---|---|
| `CI-GIM-001` | `cargo fmt --check` | Rust formatting is stable. |
| `CI-GIM-002` | `cargo check --all-targets` | Rust code and all test targets compile. |
| `CI-GIM-003` | `cargo clippy --all-targets -- -D warnings` | Clippy policies pass with new schema/result types. |
| `CI-GIM-004` | `cargo test -p daemon --bin daemon` | Daemon unit tests, including rollup additions, pass. |
| `CI-GIM-005` | `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done` | Every daemon integration target runs, including `installation_labour_engine`. |
| `CI-GIM-006` | `(cd packages/intelligence && bun run test)` | New and changed TypeScript tests pass. |
| `CI-GIM-007` | `bun run test` | Workspace TypeScript/package tests pass. |
| `CI-GIM-008` | `/agent-browser` desktop/web verification flow | Rendered operator surfaces expose labour/permit context with real UI. |

## 5. Shared Test Infrastructure

| Harness or fixture | Owner surface | Requirements |
|---|---|---|
| `GoodsQualificationHarness` | `goods-qualification.test.ts` | Uses the real `qualifyGoodsOpportunity()` path, real CPV/legal-category logic, real `computeAddressableValue()` source. |
| `DescriptionQualityTruthTable` | new `description-quality.test.ts` | Uses the real `gateDescriptionQuality()` union. Adds `goods_led_mixed`; no side-channel verdict taxonomy. |
| `CoverageRollupHarness` | `coverage-rollup.test.ts` | Uses real basis-point math and existing conservative null handling style from `TS-COV-005`. |
| `L4TestStateDb` and `TestStateDb` | daemon integration tests | All SQLite tests use shared support helpers. No bespoke temp DB setup outside support helpers. |
| `InstallationLabourEngineHarness` | new `crates/daemon/tests/installation_labour_engine.rs` | Mirrors `customs_fx_freight_engine.rs`: schema epoch assert, migration assert, real row seeding, direct pence assertions. |
| `DecisionBasisHashHarness` | `decision_basis_hash.rs` | Extends `seed_candidate_quote_and_landed_cost()` and `assert_single_vector_mutation_changes_hash()` for labour and permit fields. |
| `LabourSkillCanaryCorpus` | `tests/fixtures/calibration/landed-cost/uk-installation-labour/` | Requires `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`, `metadata.json`, `manifest.md`; `metadata.json.row_count` must equal the seven JSONL rows and `rubric_version` is asserted. |
| `RateRefreshSkillHarness` | `skills/source-health/uk-installation-labour-rate-refresh/` | Source-health-style quarterly refresh skill canary. Produces candidate JSON updates with provenance and changed-cell evidence; never mutates production reference files without reviewed update path. |
| `LabourSkillLintHarness` | `skill_lint.rs` | Checks worker frontmatter, `http_public`, required sections, warning codes, run-manifest tokens, and no CDP allowlist membership. |
| `OverlayLabourCostHarness` | `skills/validation/overlay-labour-cost/` | Runs real overlay canary/golden assertions with `headless_contract`, including outage/rejection -> review. |
| `AgentBrowserUiHarness` | `/agent-browser` | Drives the real desktop/web UI to inspect manifest viewer and contract card text. |

## 6. Planned Test File Inventory

| File | Status | Primary IDs |
|---|---|---|
| `packages/intelligence/src/qualification/goods-qualification.test.ts` | extend | `TS-QUAL-003a`, `TS-QUAL-003b`, `TS-QUAL-006`, `TS-QUAL-007`, `TS-QUAL-008` |
| `packages/intelligence/src/customs/description-quality.test.ts` | new | `TS-DQ-001` through `TS-DQ-008` |
| `packages/intelligence/src/margins/coverage-rollup.test.ts` | extend | `TS-COV-007`, `TS-COV-008`, `TS-COV-009` |
| `packages/contracts-schema/src/index.ts` tests/build | extend | `TS-SCHEMA-001` through `TS-SCHEMA-005` |
| daemon bin tests in touched rollup modules | extend | `RS-ROLLUP-001` through `RS-ROLLUP-005` |
| `crates/daemon/tests/decision_basis_hash.rs` | extend | `RS-HASH-001` through `RS-HASH-003` |
| `crates/daemon/tests/customs_fx_freight_engine.rs` | preserve/extend schema assert | `RS-SCHEMA-001` |
| `crates/daemon/tests/installation_labour_engine.rs` | new | `E2E-IL-001` through `E2E-IL-011`, `GOLD-L0-001`, `NEG-CTRL-001` through `NEG-CTRL-003` |
| `crates/daemon/tests/skill_lint.rs` | extend | `SKL-LINT-001` through `SKL-LINT-005` |
| `crates/daemon/tests/skill_canary_harness.rs` | extend | `CAN-LAB-001` through `CAN-LAB-007`, `CAN-WARN-001`, `CAN-OVERLAY-001`, `CAN-RATE-REFRESH-001` |
| `skills/source-health/uk-installation-labour-rate-refresh/` lint tests | new/extend | `SKL-RATE-REFRESH-001`, `SKL-RATE-REFRESH-002`, `SKL-RATE-REFRESH-003` |
| live smoke target | new/nightly | `SMOKE-LIVE-001` |
| agent-browser UI evidence | new | `UI-AGB-001` through `UI-AGB-003` |
| research-spike beads | future | `SPIKE-Z-001` through `SPIKE-Z-003` |

## 7. Detailed Test Catalog

### 7.1 Qualification And Service Gates

| Test ID | Tier | Test |
|---|---|---|
| `TS-QUAL-003a` | T1 | Separable mixed with `totalValuePence=900000`, `goodsAddressableValuePence=540000`, `goodsSeparability=true`, and `accessRoute="open_tender"` returns `pass` because contract-value share is `6000` bps and meets `GOODS_DOMINANCE_FLOOR_BPS`. |
| `TS-QUAL-003b` | T1 | Separable mixed with `totalValuePence=900000`, `goodsAddressableValuePence=360000`, `goodsSeparability=true` returns `review`, not the old `pass`, because `4000` bps is below the dominance floor. |
| `TS-QUAL-006` | T1 | `mixed` and `works` inputs with `goodsSeparability !== false`, `goodsDensityScore >= GOODS_PRESENCE_FLOOR_BPS`, and no high-confidence no-list hit return `review`, proving Phase-1 provisional admission. |
| `TS-QUAL-007` | T1 | A goods-dominant bespoke/no-list case containing `bespoke` or `made-to-measure` returns `review` with reason `out_of_scope_bespoke`; the same value-share without the flag would otherwise pass. |
| `TS-QUAL-008` | T1 | When `goodsAddressableValuePence` and `totalValuePence` are present, contract-value share is preferred over cost proxy; when the numerator is absent, Phase 1 returns `review` and never computes landed-cost proxy math. |

### 7.2 Description-Quality Truth Table

The new file is `packages/intelligence/src/customs/description-quality.test.ts`. It mirrors the compact `bun:test` style of `customs-classification.test.ts`.

| Test ID | Description | Goods head noun | Service term | Expected verdict |
|---|---|---|---|---|
| `TS-DQ-001` | `Supply and installation of steel shelving` | `shelving` | `installation` | `goods_led_mixed`, with `goodsHeadNoun="shelving"`, `supplySignal="supply"`, and `installationTerms` containing `installation` |
| `TS-DQ-002` | `Supply of maintenance kits` | `kits` | `maintenance` as goods qualifier | `accept` |
| `TS-DQ-003` | `Supply, install and maintain festive lighting` | `lighting` | `install`, `maintain` | `goods_led_mixed` |
| `TS-DQ-004` | `Installation services` | none | `installation` | `reject_service` |
| `TS-DQ-005` | `Provision of grounds maintenance and grass cutting` | none | `maintenance` | `reject_service` |
| `TS-DQ-006` | `Managed cleaning service; contractor supplies materials` | weak/trivial | `managed`, `service` | `reject_service` |
| `TS-DQ-007` | `Reactive and planned electrical maintenance across estate` | none | `maintenance`, `repair` | `reject_service` |
| `TS-DQ-008` | `Consultancy and training` | none | `consultancy`, `training` | `reject_service` |

### 7.3 Coverage Rollup And Goods-Share Metrics

| Test ID | Tier | Test |
|---|---|---|
| `TS-COV-007` | T1 | Computes `goods_value_share_bps = round(goods_landed_cost_gbp_pence * 10000 / (goods_landed_cost_gbp_pence + installation_labour_cost_gbp_pence + permit_traffic_mgmt_cost_gbp_pence))`; with goods `1000000`, labour `240000`, permits `200000`, expected share is `6944`. |
| `TS-COV-008` | T1 | Permits/TM push share down: a high-street case that passes before permits (`round(1000000*10000/(1000000+240000))=8065`) becomes `6944` after permits and routes according to the all-in denominator. A permit-heavy boundary case below floor routes `review` when near band or `reject` when clearly below. |
| `TS-COV-009` | T1 | Adds `installation_labour_coverage_bps`, `installation_labour_value_coverage_bps`, and explicit permit/TM coverage behavior. Missing labour estimates return conservative `0` coverage and force review, mirroring `TS-COV-005` null handling. |

### 7.4 Contracts Schema And Type Coverage

| Test ID | Tier | Test |
|---|---|---|
| `TS-SCHEMA-001` | T1 | `ProductCandidateRow` exposes `InstallationRequirement` through `critical_attributes_json` typing or additive fields with all spec keys. |
| `TS-SCHEMA-002` | T1 | `LandedCostEstimateRow` exposes goods-only subtotal, labour low/base/high/confidence, permit/TM low/base/high, and permit responsibility fields. |
| `TS-SCHEMA-003` | T1 | `PipelineAssessmentRow` exposes `goods_value_share_bps`, `goods_value_share_basis`, and labour/permit coverage fields. |
| `TS-SCHEMA-004` | T1 | `SupplierQuoteRow.quote_class` still accepts the Tier-B extension point value `installer_quote` without implementing Tier-B behavior. |
| `TS-SCHEMA-005` | T1 | `MaintenancePeriodEstimateRow` or equivalent first-class type exists; recurring maintenance fields are not hidden inside generic untyped `Json`. |

### 7.5 Rust Rollup, Schema, Persistence, And Hashing

| Test ID | Tier | Test |
|---|---|---|
| `RS-ROLLUP-001` | T1 | `build_domestic_landed_cost_estimate` adds labour base and permit/TM base to supplier-cost base. Example: supplier `800000`, labour `120000`, permits `30000` -> economic landed cost `950000`. |
| `RS-ROLLUP-002` | T1 | `build_import_landed_cost_estimate` adds labour base and permit/TM base after supplier, freight, duty, and import VAT. Example: supplier `700000`, freight `100000`, duty `50000`, import VAT `170000`, labour `240000`, permits `200000` -> goods subtotal `1020000`, economic landed cost `1460000`. |
| `RS-ROLLUP-003` | T1 | Low/high net margin bands use labour and permit low/base/high consistently; base +/-20 percent defaults are preserved where no narrower evidence exists. |
| `RS-ROLLUP-004` | T1 | Pure-goods candidate with no `InstallationRequirement` preserves labour and permits as `0` or `NULL`, and economic landed cost equals goods-only subtotal. |
| `RS-ROLLUP-005` | T1 | Saturating integer-pence arithmetic is used for labour and permit additions. |
| `RS-SCHEMA-001` | T2 | `customs_fx_freight_engine.rs` keeps passing with direct `landed_cost_estimates` INSERT statements that omit new labour/permit columns; `DEFAULT_SCHEMA_EPOCH` and SQLite `user_version` are `10`. |
| `RS-SCHEMA-002` | T2 | `migrations/0014_installation_labour_fields.sql` is applied and present in `schema_migrations`; all added direct-INSERT-sensitive columns are nullable or `DEFAULT 0`. |
| `RS-SCHEMA-003` | T2 | Supabase projection migration `infra/supabase/migrations/0010_*.sql` projects new `landed_cost_estimates_viewer` and `pipeline_assessments` fields, and `schema_metadata` is epoch `10`. |
| `RS-SEED-001` | T2 | `installation_labour_engine.rs` defines and uses a local `landed_cost_insert_sql()` helper with an explicit column list, mirroring `customs_fx_freight_engine.rs`; deterministic scenario rows are direct-seeded through that helper before real rollup/gate assertions. |
| `RS-HASH-001` | T2 | Decision-basis vector includes goods-only subtotal, labour base/low/high/confidence, permit/TM base/low/high/responsibility, goods-share basis, and goods-share bps. |
| `RS-HASH-002` | T2 | `assert_single_vector_mutation_changes_hash()` changes the hash when any labour or permit field mutates. |
| `RS-HASH-003` | T2 | Historical rows with null/default labour and permit fields hash deterministically and produce a lowercase 64-character SHA-256 string. |

### 7.6 Skill Lint And Canary Coverage

| Test ID | Tier | Test |
|---|---|---|
| `SKL-LINT-001` | T2 | `skills/landed-cost/uk-installation-labour-estimate/SKILL.md` frontmatter declares `skill_id: landed-cost/uk-installation-labour-estimate`, `skill_version`, `interaction_profile: http_public`, `result_schema_version`, `model_profile`, `requires_cdp: false`, `side_effect_class: none`, and `agent_cli: claude`. |
| `SKL-LINT-002` | T2 | The worker skill is absent from `CDP_BROWSER_SKILLS` and `SIGNED_IN_BROWSER_SKILLS`; it does not document `bin/ensure-cdp-9222` or CDP browser membership. |
| `SKL-LINT-003` | T2 | `SKILL.md` contains `## Non-goals`, `## Failure Codes`, `## Resume Strategy`, and all run-manifest tokens required by `RUN_MANIFEST_REQUIRED_TOKENS`. |
| `SKL-LINT-004` | T2 | `## Failure Codes` enumerates all eight warning codes: `LABOUR_REGION_UNKNOWN`, `LABOUR_SCOPE_UNRESOLVED`, `ACCESS_AT_HEIGHT_ASSUMED_MEWP`, `MAINTENANCE_TERM_ASSUMED`, `RATE_TABLE_STALE`, `PERMIT_RESPONSIBILITY_UNKNOWN`, `LANE_RENTAL_ZONE`, `TTRO_LEAD_TIME_RISK`. |
| `SKL-LINT-005` | T2 | The worker skill requires `SKILL.md`, `result.schema.json`, `canary.inputs.jsonl`, and `golden.assertions.yaml`; it does not require `checkpoint.schema.json`. |
| `CAN-LAB-001` | T2 | `lights-nw-2day-01` asserts labour base within `210000..=265000`, trades `{electrician, labourer}`, operated boom MEWP, `permit_responsibility=unknown`, and `PERMIT_RESPONSIBILITY_UNKNOWN`. |
| `CAN-LAB-002` | T2 | `lights-lon-2day-01` asserts London multiplier: labour base equals NW base times `14000/9300` within +/-1 percent, and Regent Street variant emits `LANE_RENTAL_ZONE` plus `TTRO_LEAD_TIME_RISK`. |
| `CAN-LAB-003` | T2 | `cctv-yh-install-01` resolves security/electrical installer, applies YH multiplier `8800`, uses MEWP only for at-height mounts, and differs from Scenario 1 trade set. |
| `CAN-LAB-004` | T2 | `furniture-lon-assembly-01` resolves fitter/handyperson and asserts no access-equipment line item. |
| `CAN-LAB-005` | T2 | `region-unknown-01` falls back to multiplier `1.0`, emits `LABOUR_REGION_UNKNOWN`, and still returns `labour_status: estimated`. |
| `CAN-LAB-006` | T2 | `scope-unresolved-01` emits `LABOUR_SCOPE_UNRESOLVED`, lowers `labour_confidence_bps`, and routes to review without crashing. |
| `CAN-LAB-007` | T2 | `rate-stale-01` emits `RATE_TABLE_STALE` and still produces a three-point estimate. |
| `CAN-WARN-001` | T2 | The canary/golden corpus and Scenario 4 E2E cover all eight warning codes with no unpinned code. |
| `CAN-OVERLAY-001` | T2 | `skills/validation/overlay-labour-cost` lints and canary-runs as `headless_contract`; validator rejection or outage forces review, not pass or crash. |
| `SKL-RATE-REFRESH-001` | T2 | `skills/source-health/uk-installation-labour-rate-refresh/` lints as a source-health-style enrichment skill with `SKILL.md`, `result.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, and `validator-rubric.yaml`. |
| `SKL-RATE-REFRESH-002` | T2 | The refresh skill contract enumerates the four candidate output families: labour rates, access-equipment bands, installation profiles, and permit/TM bands. |
| `SKL-RATE-REFRESH-003` | T2 | The refresh skill declares no production reference-file mutation without an explicit reviewed update path. |
| `CAN-RATE-REFRESH-001` | T2 | Refresh skill canary feeds stale `2026-q2` reference inputs and asserts candidate updated JSON files, source URLs, `as_of`, changed cells, provenance, and no in-place mutation of `skills/landed-cost/uk-installation-labour-estimate/reference/*.json`. |

### 7.7 Appendix-E Deterministic E2E Manifests

All `E2E-IL-*` tests live in `crates/daemon/tests/installation_labour_engine.rs`. Each test seeds real SQLite rows and runs the real rollup/gate path. Every case asserts integer pence and reconstructs:

`goods_value_share_bps = round(goods_landed_cost_gbp_pence * 10000 / (goods_landed_cost_gbp_pence + installation_labour_cost_gbp_pence + permit_traffic_mgmt_cost_gbp_pence))`

| Test ID | Scenario | Seeded pence and bps assertions | Expected verdict |
|---|---|---|---|
| `E2E-IL-001` | Festive lights supply/install/de-install, NW | goods `1000000`; labour base `240000` with hard band `210000..=265000`; permit base `200000`; permit high `340000`; economic `1440000`; pre-permit share `8065`; persisted all-in share `6944`; warnings include `ACCESS_AT_HEIGHT_ASSUMED_MEWP` and `PERMIT_RESPONSIBILITY_UNKNOWN`. | `review`, because unresolved permit responsibility and public-realm assumptions require operator review |
| `E2E-IL-002` | Office/classroom furniture supply/install, London | goods `2400000`; labour `180000`; permit `0`; economic `2580000`; goods share `9302`; no access-equipment rows; LON fitter/handyperson rate cell asserted. | `pass`, because the deterministic fixture is open-route, goods-dominant, and has no unresolved access-equipment or permit assumption |
| `E2E-IL-003` | Wooden shelving/racking supply/fit | goods `900000`; labour `220000`; permit `0`; economic `1120000`; goods share `8036`; joiner trade asserted; metal twin asserts non-zero `customs_duty_gbp_pence`. | `review` due works/division-45 conservatism |
| `E2E-IL-004` | Playground equipment supply/install/maintenance | goods `1800000`; install labour `450000`; permit `50000`; economic capital event `2300000`; goods share `7826`; maintenance row has visits/year `4`, term `3`, per-visit `60000`, annual `240000`, visible term `720000`; denominator excludes `720000`. | `review`, because recurring maintenance remains operator-visible even though capital event dominance holds |
| `E2E-IL-005` | Grounds maintenance services | stage 11 row count `0`; no landed-cost pence row is created; service gate asserts no goods head noun. | `reject` at service gate |
| `E2E-IL-006` | Managed cleaning service with trivial consumables | goods `50000`; labour `950000`; permit `0`; economic `1000000`; goods share `500`; reject reason is dominance floor, not missing data. | `reject` as economically service-only |
| `E2E-IL-007` | Electrical maintenance/repair | stage 11 row count `0`; no labour estimate is invoked; service gate returns `reject_service`. | `reject` at service gate |
| `E2E-IL-008` | Pure A4 copier paper | goods `120000`; labour `0` or `NULL`; permit `0` or `NULL`; economic `120000`; goods share `10000`; no `InstallationRequirement`. | `pass` unchanged |
| `E2E-IL-009` | CCTV camera supply/install | goods `1500000`; labour `420000`; permit `0`; economic `1920000`; goods share `7813`; YH multiplier `8800`; resolver trade differs from Scenario 1. | `review`, because at-height mounting and cabling assumptions require operator review |
| `E2E-IL-010` | Traffic signal supply/installation | goods `600000`; labour `900000`; permit `400000`; economic `1900000`; goods share `3158`; reject is due to Phase-2 dominance floor; resolver confidence may be lower but category gate is not the cause. | `reject` |
| `E2E-IL-011` | Bespoke made-to-measure windows | goods `3000000`; labour `400000`; permit `0`; economic `3400000`; goods share `8824`; control twin without bespoke flag would pass; real case emits `out_of_scope_bespoke`. | `review` flagged out of scope |

Negative-control dual assertions:

| Test ID | Description-quality assertion | Integration assertion |
|---|---|---|
| `NEG-CTRL-001` | Scenario 5 maps to `TS-DQ-005` and `reject_service`. | `E2E-IL-005` asserts no candidate reaches Stage 11 and no landed-cost row exists. |
| `NEG-CTRL-002` | Scenario 6 maps to `TS-DQ-006` and `reject_service`. | `E2E-IL-006` asserts the controlled dominance-floor twin has goods share `500` bps and rejects if it reaches Phase 2. |
| `NEG-CTRL-003` | Scenario 7 maps to `TS-DQ-007` and `reject_service`. | `E2E-IL-007` asserts no labour estimate is invoked and no Stage 11 row exists. |

### 7.8 Live Loop, Gold-Set, And Promotion

| Test ID | Tier | Test |
|---|---|---|
| `SMOKE-LIVE-001` | T4 | Seed an in-scope process and run `run_landed_cost_live_loop`; assert the labour skill is transient-loaded from `skills/landed-cost/uk-installation-labour-estimate`, job kind `landed_cost_uk_installation_labour` executes after FX/supplier basis and before landed-cost synthesis, and no browser/CDP dependency is invoked. |
| `GOLD-L0-001` | T2 | Appendix A 11-scenario corpus gate: Scenarios 5, 6, 7, and 8 are 100 percent correct; acceptance precision across Scenarios 1, 2, 3, 4, and 9 meets the agreed floor; Scenarios 10 and 11 enforce boundary behavior. |
| `GOLD-L0-002` | T2 | `docs/strategy/layers/layer-0-gold-set-evaluation.md` section 1.6 contains the low-reg category expansion, metal shelving split, no-list exclusions, and labelled gold-set slices. |
| `GOLD-L0-003` | T2 | Missing `validator-rubric.yaml` in the labour canary fixture or overlay remains a stable-production blocker. |

### 7.9 UI And Operator Surfacing

| Test ID | Tier | Test |
|---|---|---|
| `UI-AGB-001` | T3 | Using `/agent-browser`, open a Scenario 1 `review` manifest in the desktop control panel and assert a cost table row named `Installation labour` with low/base/high/confidence values. |
| `UI-AGB-002` | T3 | In the same manifest, assert a separate `Permits & traffic management` row with low/base/high and `permit_responsibility`, plus warning codes, goods-share basis, floor, and decision route. |
| `UI-AGB-003` | T3 | In `contract-card-frame.tsx`, assert `labourCoverageBps` renders for review cards and the reason list can show near-floor cost proxy, unresolved permit responsibility, no-list/bespoke, or labour uncertainty. Web read-only views expose no mutation controls. |

### 7.10 Tier-B Research-Spike Coverage

These are tracked coverage entries, not code tests.

| Test ID | Requirement | Research-spike deliverable |
|---|---|---|
| `SPIKE-Z-001` | `FR-Z.1` | Research output and extension-point design for operator-triggered live per-estimate enrichment lane, including basis marker and rerun contract. |
| `SPIKE-Z-002` | `FR-Z.2` | Research output and extension-point design for `SupplierQuoteRow.quote_class = "installer_quote"` and override/calibration semantics. |
| `SPIKE-Z-003` | `FR-Z.3` | Research output and extension-point design for per-authority/per-zone permit and lane-rental sub-model beyond Tier-A banded permits/TM. |

## 8. Warning-Code Coverage

| Warning code | Required test/case |
|---|---|
| `LABOUR_REGION_UNKNOWN` | `CAN-LAB-005` / `region-unknown-01` |
| `LABOUR_SCOPE_UNRESOLVED` | `CAN-LAB-006` / `scope-unresolved-01` |
| `ACCESS_AT_HEIGHT_ASSUMED_MEWP` | `CAN-LAB-001`, `CAN-LAB-003` |
| `MAINTENANCE_TERM_ASSUMED` | `E2E-IL-004` Scenario 4 maintenance manifest |
| `RATE_TABLE_STALE` | `CAN-LAB-007` / `rate-stale-01` |
| `PERMIT_RESPONSIBILITY_UNKNOWN` | `CAN-LAB-001` / `lights-nw-2day-01` |
| `LANE_RENTAL_ZONE` | `CAN-LAB-002` / London Regent Street variant |
| `TTRO_LEAD_TIME_RISK` | `CAN-LAB-002` / London Regent Street variant |

## 9. Coverage Matrix

| Spec Requirement | Tier | Test ID(s) | Status |
|---|---|---|---|
| `FR-Q.1` | A | `TS-QUAL-006`, `E2E-IL-001`, `E2E-IL-003`, `E2E-IL-009`, `E2E-IL-010`, `E2E-IL-011` | Covered |
| `FR-Q.2` | A | `TS-QUAL-003a`, `TS-QUAL-003b`, `TS-QUAL-008` | Covered |
| `FR-Q.3` | A | `TS-COV-007`, `TS-COV-008`, `E2E-IL-001`, `E2E-IL-006`, `E2E-IL-010`, `E2E-IL-011` | Covered |
| `FR-Q.4` | A | `TS-DQ-001`, `TS-DQ-002`, `TS-DQ-003`, `TS-DQ-004`, `TS-DQ-005`, `TS-DQ-006`, `TS-DQ-007`, `TS-DQ-008`, `E2E-IL-005`, `E2E-IL-007` | Covered |
| `FR-Q.5` | A | `TS-QUAL-007`, `E2E-IL-011`, `UI-AGB-003` | Covered |
| `FR-Q.6` | A | `TS-COV-007`, `TS-COV-008`, `RS-HASH-001`, `RS-HASH-002`, `E2E-IL-001`, `E2E-IL-010` | Covered |
| `FR-L.1` | A | `E2E-IL-001`, `E2E-IL-004`, `E2E-IL-008`, `CAN-LAB-005`, `TS-SCHEMA-001` | Covered |
| `FR-L.2` | A | `SKL-LINT-001`, `SKL-LINT-002`, `SKL-LINT-003`, `SKL-LINT-004`, `SKL-LINT-005`, `SMOKE-LIVE-001` | Covered |
| `FR-L.3` | A | `CAN-LAB-001`, `CAN-LAB-002`, `CAN-LAB-003`, `CAN-LAB-004`, `CAN-LAB-005`, `CAN-LAB-006`, `CAN-LAB-007`, `TS-SCHEMA-002` | Covered |
| `FR-L.4` | A | `CAN-LAB-007`, `SKL-LINT-005`, `SMOKE-LIVE-001`, `GOLD-L0-003` | Covered |
| `FR-L.5` | A | `CAN-LAB-001`, `CAN-LAB-003`, `E2E-IL-001`, `E2E-IL-009`, `E2E-IL-010`, `E2E-IL-011`, `CAN-OVERLAY-001` | Covered |
| `FR-L.6` | A | `E2E-IL-001`, `CAN-LAB-001`, `RS-ROLLUP-002` | Covered |
| `FR-L.7` | A | `E2E-IL-001`, `E2E-IL-010`, `CAN-LAB-001`, `CAN-LAB-002`, `UI-AGB-002` | Covered |
| `FR-L.8` | A | `E2E-IL-004`, `TS-SCHEMA-005`, `CAN-WARN-001` | Covered |
| `FR-L.9` | A | `CAN-LAB-001`, `CAN-LAB-003`, `CAN-LAB-004`, `E2E-IL-001`, `E2E-IL-009` | Covered |
| `FR-L.10` | A | `CAN-LAB-007`, `SKL-RATE-REFRESH-001`, `SKL-RATE-REFRESH-002`, `SKL-RATE-REFRESH-003`, `CAN-RATE-REFRESH-001` | Covered |
| `FR-L.11` | A | `CAN-OVERLAY-001`, `CAN-LAB-006`, `SKL-LINT-005` | Covered |
| `FR-L.12` | A | `CAN-WARN-001`, `CAN-LAB-005`, `CAN-LAB-006`, `CAN-LAB-007`, `E2E-IL-004` | Covered |
| `FR-R.1` | A | `RS-SCHEMA-001`, `RS-SCHEMA-002`, `RS-ROLLUP-004`, `TS-SCHEMA-002`, `E2E-IL-008` | Covered |
| `FR-R.2` | A | `RS-ROLLUP-001`, `RS-ROLLUP-002`, `RS-ROLLUP-003`, `RS-ROLLUP-004`, `RS-ROLLUP-005`, `E2E-IL-001`, `E2E-IL-008` | Covered |
| `FR-R.3` | A | `SMOKE-LIVE-001`, `SKL-LINT-002`, `E2E-IL-001`, `E2E-IL-008` | Covered |
| `FR-R.4` | A | `TS-COV-007`, `TS-COV-008`, `TS-COV-009`, `TS-SCHEMA-003`, `E2E-IL-001`, `E2E-IL-006`, `E2E-IL-010` | Covered |
| `FR-R.5` | A | `TS-SCHEMA-001`, `TS-SCHEMA-002`, `TS-SCHEMA-003`, `TS-SCHEMA-004`, `TS-SCHEMA-005` | Covered |
| `FR-R.6` | A | `RS-SCHEMA-001`, `RS-SCHEMA-002`, `RS-SCHEMA-003`, `RS-SEED-001`, `CI-GIM-001`, `CI-GIM-002`, `CI-GIM-003`, `CI-GIM-004`, `CI-GIM-005` | Covered |
| `FR-R.7` | A | `RS-HASH-001`, `RS-HASH-002`, `RS-HASH-003` | Covered |
| `FR-U.1` | A | `UI-AGB-001`, `UI-AGB-002`, `E2E-IL-001`, `CAN-WARN-001` | Covered |
| `FR-U.2` | A | `UI-AGB-003`, `TS-COV-009`, `E2E-IL-001`, `E2E-IL-011` | Covered |
| `FR-V.1` | A | `E2E-IL-001`, `E2E-IL-002`, `E2E-IL-003`, `E2E-IL-004`, `E2E-IL-005`, `E2E-IL-006`, `E2E-IL-007`, `E2E-IL-008`, `E2E-IL-009`, `E2E-IL-010`, `E2E-IL-011`, `GOLD-L0-001` | Covered |
| `FR-V.2` | A | `E2E-IL-001`, `E2E-IL-002`, `E2E-IL-003`, `E2E-IL-004`, `E2E-IL-005`, `E2E-IL-006`, `E2E-IL-007`, `E2E-IL-008`, `E2E-IL-009`, `E2E-IL-010`, `E2E-IL-011` | Covered |
| `FR-V.3` | A | `CAN-LAB-001`, `CAN-LAB-002`, `CAN-LAB-003`, `CAN-LAB-004`, `CAN-LAB-005`, `CAN-LAB-006`, `CAN-LAB-007`, `CAN-WARN-001`, `GOLD-L0-003` | Covered |
| `FR-V.4` | A | `GOLD-L0-001`, `GOLD-L0-002`, `E2E-IL-003`, `E2E-IL-008`, `E2E-IL-011` | Covered |
| `FR-V.5` | A | `CI-GIM-001`, `CI-GIM-002`, `CI-GIM-003`, `CI-GIM-004`, `CI-GIM-005`, `CI-GIM-006`, `CI-GIM-007`, `CI-GIM-008` | Covered |
| `FR-Z.1` | B | `SPIKE-Z-001` | Research-spike, no code test |
| `FR-Z.2` | B | `SPIKE-Z-002` | Research-spike, no code test |
| `FR-Z.3` | B | `SPIKE-Z-003` | Research-spike, no code test |

## 10. Success-Criteria Coverage Summary

| Research Criterion | Named Tests |
|---|---|
| Goods-led mixed shelving reaches pass/review | `TS-DQ-001`, `E2E-IL-003` |
| Grounds maintenance services still rejected | `TS-DQ-005`, `E2E-IL-005` |
| Pure A4 copier paper still passes unchanged | `E2E-IL-008`, `RS-ROLLUP-004` |
| In-scope install has non-zero 3-point labour band and confidence | `E2E-IL-001`, `CAN-LAB-001` |
| Goods value-share threshold is config, not hardcoded | `TS-QUAL-003a`, `TS-QUAL-003b`, `TS-COV-008` |
| Region derived from location changes labour estimate | `CAN-LAB-002`, `CAN-LAB-003`, `CAN-LAB-005`, `E2E-IL-002`, `E2E-IL-009` |
| Recurring maintenance produces multi-period estimate | `E2E-IL-004`, `TS-SCHEMA-005` |
| Screening-grade +/-20 percent, ex-VAT, not firm quote | `CAN-LAB-001`, `CAN-LAB-007`, `UI-AGB-002` |
| Labour skill failures fail-soft and route review | `CAN-LAB-006`, `CAN-OVERLAY-001` |
| New region/rate quarter is JSON edit plus version bump | `CAN-LAB-007`, `SKL-LINT-005`, `SKL-RATE-REFRESH-001`, `CAN-RATE-REFRESH-001` |

## 11. Downstream Consumer Notes

- Strategy must reference the stable IDs in this document for phase gates.
- Beads must carry the relevant test IDs in their acceptance criteria.
- A completion audit should mechanically grep the implementation for each ID and compare the concrete assertions here against the actual tests.
- Any future refinement of Tier-B scope must first replace `SPIKE-Z-*` research coverage with real requirement IDs and named tests.
