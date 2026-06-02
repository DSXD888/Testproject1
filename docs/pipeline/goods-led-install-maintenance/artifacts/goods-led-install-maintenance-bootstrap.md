# Goods-Led Install/Maintenance + UK Labour-Cost Engine - Test Bootstrap

> Document Type: TDD Bootstrap
> Feature: Goods-Led Supply-and-Install / Maintenance Expansion + UK Labour-Cost Engine
> Created: 2026-06-02
> TDD: `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md`

This bootstrap document explains how to implement and run the tests defined in the TDD. It does not replace the spec. If there is a conflict, the spec wins.

## 1. Bootstrap Rules

1. Tests must use the real code paths named here: real SQLite seeding, real rollup/gate functions, real skill lint, real skill canary harness, and real UI browser flow.
2. Do not use mocks, HTTP stubs, fake rendered components, skipped tests, or existence-only assertions.
3. Do not narrow the gate to only `cargo test -p daemon --bin daemon`.
4. Keep legacy direct-INSERT integration tests green by making all new landed-cost columns nullable or `DEFAULT 0`.
5. The new `description-quality.test.ts` file is mandatory and must include all eight truth-table rows.

## 2. Prerequisites

| Tool | Required use |
|---|---|
| Rust nightly from `rust-toolchain.toml` | Daemon checks, clippy, unit, and integration tests. |
| Cargo | Rust package manager. Do not use npm/yarn/pnpm for Rust. |
| Bun `1.3.10` from `package.json` | TypeScript package tests. Do not use npm/yarn/pnpm. |
| `rg` | Fast test/file inspection. |
| `/agent-browser` skill | Real desktop/web UI verification for manifest and contract-card checks. |
| SQLite support helpers | `TestStateDb` and `L4TestStateDb` from `crates/daemon/tests/support/db.rs`. |

Prerequisite validation commands:

```bash
rustc --version
cargo --version
bun --version
rg --version
```

## 3. Full Stage Gate

This is the minimum green bar for any stage touching this feature:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
bun test
```

Feature-focused fast loop during implementation:

```bash
(cd packages/intelligence && bun test ./src/qualification/goods-qualification.test.ts)
(cd packages/intelligence && bun test ./src/customs/description-quality.test.ts)
(cd packages/intelligence && bun test ./src/margins/coverage-rollup.test.ts)
cargo test -p daemon --bin daemon
cargo test -p daemon --test installation_labour_engine
cargo test -p daemon --test skill_lint
cargo test -p daemon --test skill_canary_harness
cargo test -p daemon --test decision_basis_hash
```

## 4. Schema Epoch Bump Checklist

The feature schema epoch is `10`. All of the following must move together:

| Surface | Required update |
|---|---|
| SQLite migration | Add `migrations/0014_installation_labour_fields.sql`. |
| SQLite epoch | `crates/daemon/src/db/sqlite.rs` `DEFAULT_SCHEMA_EPOCH`: `9 -> 10`. |
| Desktop expected epoch | `apps/desktop/src-tauri/src/main.rs` `EXPECTED_SCHEMA_EPOCH`: `9 -> 10`. |
| Desktop daemon bridge | `apps/desktop/src-tauri/src/daemon_bridge.rs`: reconcile from `8 -> 10`. |
| Desktop query service | `apps/desktop/src-tauri/src/query_service.rs`: reconcile from `8 -> 10`. |
| Supabase projection migration | Add `infra/supabase/migrations/0010_*.sql`. |
| Supabase metadata | `schema_metadata` epoch: `9 -> 10`. |
| Schema test | `customs_fx_freight_engine.rs` observes `user_version == DEFAULT_SCHEMA_EPOCH` and migration `0014_installation_labour_fields`. |

SQLite column policy:

- New `landed_cost_estimates` labour/permit columns must be nullable or `DEFAULT 0`.
- Do not break existing explicit INSERT lists in `customs_fx_freight_engine.rs` and `decision_basis_hash.rs`.
- Persist `goods_landed_cost_gbp_pence` so the goods-share ratio is reconstructable from one row.

## 5. TypeScript Test Implementation Notes

### 5.1 Qualification Tests

File: `packages/intelligence/src/qualification/goods-qualification.test.ts`

Required edits:

- Replace old `TS-QUAL-003` with `TS-QUAL-003a` and `TS-QUAL-003b`.
- Add `TS-QUAL-006`, `TS-QUAL-007`, and `TS-QUAL-008`.
- Keep using `bun:test` and the existing `qualifyGoodsOpportunity()` helper path.

Required assertions:

```typescript
it("TS-QUAL-003a: separable mixed passes when early contract-value share meets dominance floor", () => {
  const result = qualifyGoodsOpportunity({
    cpvCodes: ["33100000-1", "50750000-7"],
    tenderDeadline: "2026-06-30T00:00:00.000Z",
    now: "2026-03-28T00:00:00.000Z",
    accessRoute: "open_tender",
    qualificationBasis: "lot_level",
    totalValuePence: 900_000,
    goodsAddressableValuePence: 540_000,
    goodsSeparability: true,
    goodsDensitySignals: { physicalGoodsMentions: 3, quantityMentions: 2 },
  });

  expect(result.legalCategory).toBe("mixed");
  expect(result.addressableValuePence).toBe(540_000);
  expect(result.qualificationResult).toBe("pass");
});

it("TS-QUAL-003b: separable mixed defers when early contract-value share is below dominance floor", () => {
  const result = qualifyGoodsOpportunity({
    cpvCodes: ["33100000-1", "50750000-7"],
    tenderDeadline: "2026-06-30T00:00:00.000Z",
    now: "2026-03-28T00:00:00.000Z",
    accessRoute: "open_tender",
    qualificationBasis: "lot_level",
    totalValuePence: 900_000,
    goodsAddressableValuePence: 360_000,
    goodsSeparability: true,
    goodsDensitySignals: { physicalGoodsMentions: 3, quantityMentions: 2 },
  });

  expect(result.addressableValuePence).toBe(360_000);
  expect(result.qualificationResult).toBe("review");
});
```

`TS-QUAL-006` must assert provisional `review` for `mixed` and `works` with credible goods presence. `TS-QUAL-007` must assert `out_of_scope_bespoke`. `TS-QUAL-008` must assert that contract-value share is preferred and Phase 1 does not use cost-proxy landed-cost math.

### 5.2 Description-Quality Tests

New file: `packages/intelligence/src/customs/description-quality.test.ts`

The test file must import `gateDescriptionQuality` and assert the exact eight rows:

```typescript
import { describe, expect, it } from "bun:test";

import { gateDescriptionQuality } from "./description-quality";

describe("description quality goods-led service split", () => {
  it("TS-DQ-001: supply and installation of steel shelving is goods-led mixed", () => {
    const verdict = gateDescriptionQuality({
      text: "Supply and installation of steel shelving",
    });
    expect(verdict.kind).toBe("goods_led_mixed");
    if (verdict.kind === "goods_led_mixed") {
      expect(verdict.goodsHeadNoun).toBe("shelving");
      expect(verdict.supplySignal).toBe("supply");
      expect(verdict.installationTerms).toContain("installation");
    }
  });

  it("TS-DQ-002: supply of maintenance kits remains an accepted goods description", () => {
    expect(gateDescriptionQuality({ text: "Supply of maintenance kits" }).kind).toBe("accept");
  });

  it("TS-DQ-003: festive lighting with install and maintain is goods-led mixed", () => {
    const verdict = gateDescriptionQuality({
      text: "Supply, install and maintain festive lighting",
    });
    expect(verdict.kind).toBe("goods_led_mixed");
  });

  it("TS-DQ-004: installation services rejects as service-only", () => {
    expect(gateDescriptionQuality({ text: "Installation services" }).kind).toBe("reject_service");
  });

  it("TS-DQ-005: grounds maintenance rejects as service-only", () => {
    expect(
      gateDescriptionQuality({
        text: "Provision of grounds maintenance and grass cutting",
      }).kind,
    ).toBe("reject_service");
  });

  it("TS-DQ-006: cleaning service with supplied materials rejects as service-only", () => {
    expect(
      gateDescriptionQuality({
        text: "Managed cleaning service; contractor supplies materials",
      }).kind,
    ).toBe("reject_service");
  });

  it("TS-DQ-007: electrical maintenance rejects as service-only", () => {
    expect(
      gateDescriptionQuality({
        text: "Reactive and planned electrical maintenance across estate",
      }).kind,
    ).toBe("reject_service");
  });

  it("TS-DQ-008: consultancy and training rejects as service-only", () => {
    expect(gateDescriptionQuality({ text: "Consultancy and training" }).kind).toBe(
      "reject_service",
    );
  });
});
```

Implementation must add `goods_led_mixed` to `DescriptionQualityVerdict`; do not create a parallel result channel.

### 5.3 Coverage Rollup Tests

File: `packages/intelligence/src/margins/coverage-rollup.test.ts`

Add:

- `TS-COV-007`: share arithmetic.
- `TS-COV-008`: permit/TM lowers the denominator share.
- `TS-COV-009`: labour/permit coverage metrics and conservative missing-labour behavior.

Concrete arithmetic anchor:

```typescript
expect(computeGoodsValueShareBps({
  goodsLandedCostGbpPence: 1_000_000,
  installationLabourCostGbpPence: 240_000,
  permitTrafficMgmtCostGbpPence: 200_000,
})).toBe(6_944);
```

The function name may differ, but the assertion value must not.

## 6. Rust Test Implementation Notes

### 6.1 Daemon Unit Tests

Add daemon bin tests near the rollup functions they exercise.

Required tests:

- `RS-ROLLUP-001`: domestic landed cost sums supplier + labour + permits.
- `RS-ROLLUP-002`: import landed cost sums supplier + freight + duty + import VAT + labour + permits.
- `RS-ROLLUP-003`: low/high bands use labour/permit low/high.
- `RS-ROLLUP-004`: no installation requirement keeps labour/permit zero or null.
- `RS-ROLLUP-005`: saturating pence arithmetic.

Use integer pence values from the TDD. Do not assert floats for money.

### 6.2 Decision Basis Hash

File: `crates/daemon/tests/decision_basis_hash.rs`

Extend `seed_candidate_quote_and_landed_cost()` explicit INSERT handling only after schema defaults are in place.

Required mutation tests:

```rust
#[test]
#[doc = "RS-HASH-002"]
fn rs_hash_002_hash_changes_when_installation_labour_changes() -> Result<()> {
    assert_single_vector_mutation_changes_hash(|conn| {
        conn.execute(
            "UPDATE landed_cost_estimates SET installation_labour_cost_gbp_pence = 240000 WHERE product_candidate_id = 'candidate-1'",
            [],
        )?;
        Ok(())
    })
}

#[test]
#[doc = "RS-HASH-002"]
fn rs_hash_002_hash_changes_when_permit_responsibility_changes() -> Result<()> {
    assert_single_vector_mutation_changes_hash(|conn| {
        conn.execute(
            "UPDATE landed_cost_estimates SET permit_responsibility = 'unknown' WHERE product_candidate_id = 'candidate-1'",
            [],
        )?;
        Ok(())
    })
}
```

Also add `RS-HASH-003` for null/default historical rows.

### 6.3 New Integration Target

New file: `crates/daemon/tests/installation_labour_engine.rs`

Mirror these patterns:

- `#![forbid(unsafe_code)]`.
- `mod support;`.
- Use `L4TestStateDb::new()` or `TestStateDb::new()`.
- Assert `PRAGMA user_version == DEFAULT_SCHEMA_EPOCH`.
- Assert migration `0014_installation_labour_fields` exists in `schema_migrations`.
- Seed `sources`, `contract_notices`, `notice_lots`, `product_candidates`, `supplier_quotes`, and `landed_cost_estimates` using real `rusqlite` inserts.
- Run real rollup/gate code, or call the smallest real public/internal function boundary that computes the same persisted decision state.
- Use `anyhow::ensure!` for integration assertions.

Shared helper shape:

```rust
struct InstallationLabourScenario {
    id: &'static str,
    goods_landed_cost_gbp_pence: i64,
    installation_labour_cost_gbp_pence: i64,
    permit_traffic_mgmt_cost_gbp_pence: i64,
    expected_economic_landed_cost_gbp_pence: i64,
    expected_goods_value_share_bps: i64,
    expected_decision: &'static str,
}

fn goods_share_bps(goods: i64, labour: i64, permits: i64) -> i64 {
    let denominator = goods.saturating_add(labour).saturating_add(permits);
    if denominator <= 0 {
        return 0;
    }
    ((goods * 10_000) + (denominator / 2)) / denominator
}
```

Required scenario functions:

| Test function suffix | TDD ID |
|---|---|
| `scenario_1_festive_lights_nw_hard_band` | `E2E-IL-001` |
| `scenario_2_furniture_london_no_mewp` | `E2E-IL-002` |
| `scenario_3_wooden_shelving_review_and_metal_twin_customs` | `E2E-IL-003` |
| `scenario_4_playground_maintenance_separate_period_estimate` | `E2E-IL-004` |
| `scenario_5_grounds_maintenance_rejects_before_stage_11` | `E2E-IL-005` |
| `scenario_6_cleaning_service_rejects_by_goods_share_floor` | `E2E-IL-006` |
| `scenario_7_electrical_maintenance_rejects_service_gate` | `E2E-IL-007` |
| `scenario_8_copier_paper_passes_with_zero_labour` | `E2E-IL-008` |
| `scenario_9_cctv_resolver_generalises_trade_profile` | `E2E-IL-009` |
| `scenario_10_traffic_signals_reject_by_phase_2_floor` | `E2E-IL-010` |
| `scenario_11_bespoke_windows_review_by_no_list_flag` | `E2E-IL-011` |

The Scenario 1 assertion must be literal:

```rust
ensure!(
    (210_000..=265_000).contains(&row.installation_labour_cost_gbp_pence),
    "Scenario 1 NW labour base must stay within 210000..=265000 pence"
);
```

## 7. Skill And Fixture Implementation Notes

### 7.1 Labour Worker Skill

Path: `skills/landed-cost/uk-installation-labour-estimate/`

Required files:

- `SKILL.md`
- `result.schema.json`
- `canary.inputs.jsonl`
- `golden.assertions.yaml`
- `reference/uk-regional-labour-rates-2026-q2.json`
- `reference/uk-access-equipment-bands-2026-q2.json`
- `reference/uk-installation-profiles-2026-q2.json`
- `reference/uk-permit-tm-bands-2026-q2.json`

Worker frontmatter minimum:

```yaml
skill_id: landed-cost/uk-installation-labour-estimate
skill_version: 1.0.0
interaction_profile: http_public
result_schema_version: 1.0.0
model_profile: structured_extract_low_cost
requires_cdp: false
side_effect_class: none
agent_cli: claude
```

`SKILL.md` must include:

- `## Non-goals`
- `## Failure Codes`
- `## Resume Strategy`
- `manifest_version`
- `skill_id`
- `skill_version`
- `job_id`
- `session_id`
- `execution_mode`
- `Summary`
- `Steps Taken`
- `Decisions Made`
- `Warnings`
- `Evidence`

Do not add the labour worker to `CDP_BROWSER_SKILLS` or `SIGNED_IN_BROWSER_SKILLS`.

### 7.2 Labour Canary Corpus

Path: `tests/fixtures/calibration/landed-cost/uk-installation-labour/`

Required files:

- `canary.inputs.jsonl`
- `golden.assertions.yaml`
- `validator-rubric.yaml`
- `metadata.json`
- `manifest.md`

`metadata.json` minimum:

```json
{
  "category": "uk-installation-labour",
  "fixture_surface": "calibration-corpus",
  "row_count": 7
}
```

Required case IDs:

- `lights-nw-2day-01`
- `lights-lon-2day-01`
- `cctv-yh-install-01`
- `furniture-lon-assembly-01`
- `region-unknown-01`
- `scope-unresolved-01`
- `rate-stale-01`

`manifest.md` must contain one `## Scenario` block per case. `validator-rubric.yaml` must contain `rubric_version`.

Warning-code coverage:

| Code | Case |
|---|---|
| `LABOUR_REGION_UNKNOWN` | `region-unknown-01` |
| `LABOUR_SCOPE_UNRESOLVED` | `scope-unresolved-01` |
| `ACCESS_AT_HEIGHT_ASSUMED_MEWP` | `lights-nw-2day-01`, `cctv-yh-install-01` |
| `MAINTENANCE_TERM_ASSUMED` | Scenario 4 E2E manifest |
| `RATE_TABLE_STALE` | `rate-stale-01` |
| `PERMIT_RESPONSIBILITY_UNKNOWN` | `lights-nw-2day-01` |
| `LANE_RENTAL_ZONE` | `lights-lon-2day-01` Regent Street variant |
| `TTRO_LEAD_TIME_RISK` | `lights-lon-2day-01` Regent Street variant |

### 7.3 Overlay Skill

Path: `skills/validation/overlay-labour-cost/`

Required files:

- `SKILL.md`
- `result.schema.json`
- `checkpoint.schema.json`
- `validator-rubric.yaml`
- `canary.inputs.jsonl`
- `golden.assertions.yaml`

The overlay profile is `headless_contract`. It checks hours vs scope, regional rate bounds, postcode/region plausibility, equipment proportionality, permit responsibility handling, and warning completeness. Rejection or outage forces `review`.

## 8. UI Verification

Before browser UI verification, invoke `/agent-browser`.

Required flow:

1. Start the desktop app using the repo's Bun/Tauri workflow.
2. Seed or open a Scenario 1 process routed to `review`.
3. Navigate to the manifest viewer.
4. Assert text/value presence for:
   - `Installation labour`
   - low/base/high/confidence values
   - `Permits & traffic management`
   - `permit_responsibility`
   - warning codes
   - goods-share basis, floor, and route
5. Navigate to or inspect the contract card.
6. Assert `labourCoverageBps` appears and review reasons can render:
   - near-floor cost proxy
   - unresolved permit responsibility
   - no-list/bespoke flag
   - labour uncertainty
7. Verify web read-only surfaces expose no mutation controls.

Evidence to keep:

- Browser snapshot text.
- Screenshot of manifest cost breakdown.
- Screenshot of contract card review metrics.
- Console/network errors if any are present.

## 9. Success Criteria Coverage Summary

| Subsystem | Requirement IDs | Required tests |
|---|---:|---:|
| Qualification and service gates | 6 | 16 named tests |
| Labour-cost engine | 12 | 25 named tests |
| Rollup, schema, sync, decision synthesis | 7 | 22 named tests |
| UI and operator surfacing | 2 | 3 named browser tests |
| Validation and quality gates | 5 | 19 named tests/gates |
| Tier-B research spikes | 3 | 3 research-spike deliverables |
| Total | 35 | 82 named coverage entries plus CI gates |

## 10. False Positive Prevention

| Risk | Prevention |
|---|---|
| Old `TS-QUAL-003` silently retained | Test name split requires both `TS-QUAL-003a` and `TS-QUAL-003b`; grep for old `TS-QUAL-003:` after implementation. |
| Service gate admits pure services | `TS-DQ-004` through `TS-DQ-008` must all assert `reject_service`. |
| Labour columns break old explicit INSERTs | Run `customs_fx_freight_engine.rs` and `decision_basis_hash.rs`; new columns must be omitted safely. |
| Labour not included at both rollup sites | `RS-ROLLUP-001` and `RS-ROLLUP-002` use different arithmetic anchors. |
| Goods-share denominator omits permits | `TS-COV-008` and `E2E-IL-001` compare pre-permit and all-in shares. |
| Scenario 1 rate table drift | Literal pence band `210000..=265000` fails if model/rates move outside acceptance. |
| Canary corpus shape incomplete | `skill_canary_harness.rs` asserts row count, required files, `rubric_version`, and `Scenario` mentions. |
| Browser-only skill confusion | `SKL-LINT-002` asserts no CDP allowlist membership for the labour worker. |
| Tier-B behavior fabricated too early | Coverage matrix only uses `SPIKE-Z-*` for `FR-Z.*`. |

## 11. Next Stage Kickoff

After this TDD and bootstrap are accepted, create the lightweight implementation strategy:

```text
Create a lightweight implementation strategy document (SOT #3).

- Spec: docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md
- TDD: docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md
- Bootstrap: docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-bootstrap.md

The strategy is a routing table:
- Phase ordering and dependencies.
- Domain batches.
- Gate criteria with TDD test IDs.
- Parallel execution guidance.

Output: docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-strategy.md
```
