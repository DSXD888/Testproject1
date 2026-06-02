# Goods-Led Install/Maintenance + UK Labour-Cost Engine - Architecture

> **Document Type:** Supplementary Architecture
> **Primary Source Of Truth:** `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md`
> **Created:** 2026-06-02

This document explains how to implement the specification in the existing codebase. If this document conflicts with the specification, the specification wins.

## 1. System Overview

The feature adds a goods-led mixed-contract lane and a UK labour-cost component to Stage 11 landed cost.

```text
Notice + lot + CPV + description + delivery location
  |
  v
TypeScript qualification
  - goods-qualification.ts Phase 1: provisional goods-led mixed review
  - description-quality.ts: goods_led_mixed vs reject_service
  |
  v
ProductCandidate critical_attributes_json
  - InstallationRequirement
  |
  v
Rust Stage 11 live loop
  customs -> sizing -> freight -> FX -> uk_installation_labour -> synthesis
     domestic quotes: supplier cost -> uk_installation_labour -> synthesis
  |
  v
LandedCostEstimate
  goods-only subtotal + labour + permits/TM + margin + goods_value_share_bps
  |
  v
Decision synthesis
  contract-value basis preferred; cost proxy fallback; near-floor review
  |
  v
Manifest viewer + contract card
  labour row, permit/TM row, coverage metric, warnings, review reason
```

Key constraints:

- The labour skill is transient-loaded in `run_landed_cost_live_loop`; it is not a registry source.
- The service-gate split is the highest-risk behavioural change and needs a new test file.
- Permits/TM are a separate cost component because responsibility uncertainty dominates their spread.
- Goods dominance uses contract-value share when available and cost proxy only as fallback.

## 2. Data Flow

### 2.1 Phase-1 Qualification Flow

```text
qualifyGoodsOpportunity(input)
  legalCategory = classifyLegalCategory(cpvCodes)
  goodsDensityScore = computeGoodsDensityScore(...)
  addressableValuePence = computeAddressableValue(...)
  decision = determineQualificationDecision(...)
```

Implementation changes:

- Add config values:
  - `GOODS_PRESENCE_FLOOR_BPS = 3500`
  - `GOODS_DOMINANCE_FLOOR_BPS = 6000`
- Convert `goodsDensityScore` to bps for comparisons where useful.
- Extend the input path so early contract-value share can be considered before the current density-only separable-mixed `pass`.
- If `computeAddressableValue()` cannot produce the early contract-value numerator, the separable-mixed branch returns `review`; Phase 1 never uses the landed-cost cost proxy.

Blueprint:

```typescript
const goodsPresenceFloor = config.goodsPresenceFloorBps / 10_000;
const contractValueShareBps =
  addressableValuePence !== null && totalValuePence > 0
    ? Math.round((addressableValuePence * 10_000) / totalValuePence)
    : null;

if (
  legalCategory === "mixed" &&
  goodsSeparability === true &&
  goodsDensityScore >= 0.55
) {
  if (
    contractValueShareBps !== null &&
    contractValueShareBps >= GOODS_DOMINANCE_FLOOR_BPS
  ) {
    return REVIEWABLE_ACCESS_ROUTES.has(accessRoute) ? "review" : "pass";
  }
  return "review";
}

if (
  (legalCategory === "mixed" || legalCategory === "works") &&
  goodsDensityScore >= goodsPresenceFloor &&
  goodsSeparability !== false
) {
  return "review";
}
```

### 2.2 Description Quality Flow

Current `gateDescriptionQuality()` rejects service words before item extraction. New flow:

```text
normalize text
  |
  v
detect service terms
  |
  +-- no service term -> existing bundle/single-description flow
  |
  +-- service term + goods head noun + supply signal -> goods_led_mixed
  |
  +-- service term without real goods signal -> reject_service
```

The `goods_led_mixed` verdict extends the existing union:

```typescript
export type DescriptionQualityVerdict =
  | { kind: "accept"; attributes: ExtractedAttributes }
  | { kind: "reject_generic"; reason: string; missingArchetypeFacts: string[] }
  | { kind: "reject_service"; reason: string }
  | {
      kind: "goods_led_mixed";
      installationTerms: string[];
      goodsHeadNoun: string;
      supplySignal: string;
    }
  | { kind: "reject_bundle_unsafe"; reason: string; markAs: "mixed_manual" | "gir_3_candidate" }
  | { kind: "split_safe"; subDescriptions: RawDescription[] };
```

## 3. Component Architecture

### 3.1 Installation Requirement Extractor

Owner surface: `packages/intelligence` product extraction/reclassification and `ProductCandidate.critical_attributes_json`.

Output shape:

```typescript
export interface InstallationRequirement {
  requires_installation: boolean;
  requires_maintenance: boolean;
  installation_scope:
    | "basic_assembly"
    | "fit_and_fix"
    | "on_site_commission"
    | "at_height_install"
    | "civils_groundworks";
  maintenance_schedule?: {
    visits_per_year: number;
    term_years: number;
    visit_scope: "inspect" | "service" | "repair_replace";
  };
  site_access:
    | "loading_dock"
    | "ground_level"
    | "at_height"
    | "highway_high_street"
    | "restricted";
  at_height_m?: number;
  uk_region: string;
  estimated_units?: number;
  equipment_hint?: string[];
  permit_responsibility?: "authority_provided" | "contractor_provided" | "shared" | "unknown";
  public_realm?: boolean;
  lane_rental_zone?: boolean;
}
```

Extraction is deliberately lightweight:

- CPV division/category and description terms infer the scope.
- Delivery postcode maps to region.
- Unknown region or ambiguous scope degrade confidence and route to review through warnings.

### 3.2 Installation Profile Resolver

Owner surface: labour skill.

Resolver order:

1. `uk-installation-profiles-2026-q2.json` catalogue.
2. Constrained LLM fallback inside the skill.
3. `overlay-labour-cost` sanity check.

Catalogue seed must cover:

- Flat-pack furniture.
- Wall-fixed shelving/racking.
- Standard glazing/windows.
- Festive/LED lighting.
- External signage.
- Fencing.
- Playground/outdoor.
- CCTV/security.
- Traffic signals as a labour-heavy boundary archetype rejected by economics.
- Made-to-measure windows as bespoke/no-list review.

LLM fallback cannot invent trades. It must choose from the rate table's known trades.

### 3.3 Permit And Traffic-Management Resolver

Owner surface: labour skill, with distinct persisted output fields.

Inputs:

- `public_realm`.
- `site_access`.
- `lane_rental_zone`.
- `permit_responsibility`.
- `at_height_m`.
- Tender text when available.

Policy:

- `authority_provided`: low/base/high all 0.
- `contractor_provided`: use seed bands.
- `shared`: split or conservative base/high according to reference rule.
- `unknown`: low may be 0; base is midpoint; high is conservative; emit `PERMIT_RESPONSIBILITY_UNKNOWN`.

Tier A uses banded ranges. Tier B may later refine authority/zone-specific pricing.

### 3.4 UK Installation Labour Skill

Path: `skills/landed-cost/uk-installation-labour-estimate/`.

Profile:

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

Required worker files:

- `SKILL.md`
- `result.schema.json`
- `canary.inputs.jsonl`
- `golden.assertions.yaml`
- `reference/uk-regional-labour-rates-2026-q2.json`
- `reference/uk-access-equipment-bands-2026-q2.json`
- `reference/uk-installation-profiles-2026-q2.json`
- `reference/uk-permit-tm-bands-2026-q2.json`

`checkpoint.schema.json` is not required for the worker skill.

`SKILL.md` body requirements:

- `## Non-goals`.
- `## Failure Codes` enumerating all eight warning codes from the spec.
- `## Resume Strategy`.
- Run-manifest token block: `manifest_version`, `skill_id`, `skill_version`, `job_id`, `session_id`, `execution_mode`, `Summary`, `Steps Taken`, `Decisions Made`, `Warnings`, and `Evidence`.

The worker is not a CDP skill. It must not be listed in CDP/signed-in browser allowlists, and it must not depend on `bin/ensure-cdp-9222`.

Input:

```json
{
  "process_id": "proc-1",
  "notice_id": "notice-1",
  "product_candidate_id": "candidate-1",
  "selected_quote_id": "quote-1",
  "destination_country": "GB",
  "destination_postcode": "M1 1AA",
  "installation_requirement": {},
  "goods_landed_cost_gbp_pence": 1234500,
  "rate_table_version": "2026-q2"
}
```

Output:

```json
{
  "product_candidate_id": "candidate-1",
  "selected_quote_id": "quote-1",
  "labour_status": "estimated",
  "uk_region": "NW",
  "labour_line_items": [],
  "access_equipment_line_items": [],
  "labour_base_gbp_pence": 240000,
  "labour_low_gbp_pence": 192000,
  "labour_high_gbp_pence": 288000,
  "labour_confidence_bps": 6000,
  "permit_traffic_mgmt_line_items": [],
  "permit_traffic_mgmt_base_gbp_pence": 200000,
  "permit_traffic_mgmt_low_gbp_pence": 0,
  "permit_traffic_mgmt_high_gbp_pence": 340000,
  "permit_responsibility": "unknown",
  "vat_treatment": "ex_vat_add_20pct",
  "excluded_cost_flags": [
    "not_firm_quote",
    "subcontractor_delivery_responsibility_excluded",
    "authority_resolved_permit_collapse_pending"
  ],
  "evidence": [],
  "warnings": [
    { "code": "PERMIT_RESPONSIBILITY_UNKNOWN", "message": "Permit responsibility unresolved.", "severity": "warn" }
  ],
  "blockers": []
}
```

### 3.5 MaintenancePeriodEstimate

The existing landed-cost row models a capital event. Recurring maintenance needs a separate structure to avoid overloading the capital estimate.

Recommended shape:

```typescript
export interface MaintenancePeriodEstimate {
  id: string;
  process_id: string;
  notice_id: string;
  product_candidate_id: string;
  landed_cost_estimate_id: string;
  visits_per_year: number;
  term_years: number;
  visit_scope: "inspect" | "service" | "repair_replace";
  per_visit_labour_cost_gbp_pence: number;
  annual_labour_cost_gbp_pence: number;
  visible_term_labour_cost_gbp_pence: number;
  confidence_bps: number;
  warnings: Json;
  owner_id: string;
  row_version: number;
  last_local_seq: number | null;
  stage_run_id: string | null;
  updated_at: string;
  created_at: string;
}
```

Dominance uses only the initial supply+install capital event. Maintenance appears as annualised opex and separate margin signal.

## 4. Database Schema

### 4.1 SQLite Migration

Migration: `migrations/0014_installation_labour_fields.sql`.

Sketch:

```sql
ALTER TABLE landed_cost_estimates
  ADD COLUMN goods_landed_cost_gbp_pence INTEGER DEFAULT 0;

ALTER TABLE landed_cost_estimates
  ADD COLUMN installation_labour_cost_gbp_pence INTEGER DEFAULT 0;
ALTER TABLE landed_cost_estimates
  ADD COLUMN installation_labour_low_gbp_pence INTEGER DEFAULT 0;
ALTER TABLE landed_cost_estimates
  ADD COLUMN installation_labour_high_gbp_pence INTEGER DEFAULT 0;
ALTER TABLE landed_cost_estimates
  ADD COLUMN installation_labour_confidence_bps INTEGER;

ALTER TABLE landed_cost_estimates
  ADD COLUMN permit_traffic_mgmt_cost_gbp_pence INTEGER DEFAULT 0;
ALTER TABLE landed_cost_estimates
  ADD COLUMN permit_traffic_mgmt_low_gbp_pence INTEGER DEFAULT 0;
ALTER TABLE landed_cost_estimates
  ADD COLUMN permit_traffic_mgmt_high_gbp_pence INTEGER DEFAULT 0;
ALTER TABLE landed_cost_estimates
  ADD COLUMN permit_responsibility TEXT;

ALTER TABLE pipeline_assessments
  ADD COLUMN goods_value_share_bps INTEGER;
ALTER TABLE pipeline_assessments
  ADD COLUMN goods_value_share_basis TEXT;
ALTER TABLE pipeline_assessments
  ADD COLUMN installation_labour_coverage_bps INTEGER DEFAULT 0;
ALTER TABLE pipeline_assessments
  ADD COLUMN installation_labour_value_coverage_bps INTEGER DEFAULT 0;
```

Maintenance estimate table, if implemented as a new entity:

```sql
CREATE TABLE IF NOT EXISTS maintenance_period_estimates (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL,
  notice_id TEXT NOT NULL,
  product_candidate_id TEXT NOT NULL,
  landed_cost_estimate_id TEXT NOT NULL,
  visits_per_year INTEGER NOT NULL,
  term_years INTEGER NOT NULL,
  visit_scope TEXT NOT NULL,
  per_visit_labour_cost_gbp_pence INTEGER NOT NULL DEFAULT 0,
  annual_labour_cost_gbp_pence INTEGER NOT NULL DEFAULT 0,
  visible_term_labour_cost_gbp_pence INTEGER NOT NULL DEFAULT 0,
  confidence_bps INTEGER,
  warnings TEXT NOT NULL DEFAULT '[]',
  owner_id TEXT NOT NULL DEFAULT 'local',
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

All added columns are additive and default-safe because existing integration tests insert explicit column lists.

### 4.2 Supabase Projection

Add `infra/supabase/migrations/0010_*` to project the new fields from `landed_cost_estimates_viewer`.

`pipeline_assessments` is projected by `infra/supabase/migrations/0001_projection_tables.sql` and exported through `READABLE_PROJECTION_TABLES`, so the same migration must include `goods_value_share_bps`, `goods_value_share_basis`, and labour/permit coverage fields there too.

Update Supabase `schema_metadata` to epoch 10.

### 4.3 Schema Epoch Constants

Update all of:

- `crates/daemon/src/db/sqlite.rs` `DEFAULT_SCHEMA_EPOCH`: 9 -> 10.
- `apps/desktop/src-tauri/src/main.rs` `EXPECTED_SCHEMA_EPOCH`: 9 -> 10.
- `apps/desktop/src-tauri/src/daemon_bridge.rs` default epoch: 8 -> 10.
- `apps/desktop/src-tauri/src/query_service.rs` expected epoch: 8 -> 10.
- Supabase `schema_metadata`: 9 -> 10.

## 5. Rust Integration

### 5.1 New Result Type

Add a Rust parser for the labour skill result:

```rust
#[derive(Debug, Clone, Deserialize)]
struct UkInstallationLabourResult {
    product_candidate_id: String,
    selected_quote_id: String,
    labour_status: String,
    uk_region: Option<String>,
    #[serde(default)]
    labour_line_items: Vec<Value>,
    #[serde(default)]
    access_equipment_line_items: Vec<Value>,
    #[serde(default)]
    labour_base_gbp_pence: i64,
    #[serde(default)]
    labour_low_gbp_pence: i64,
    #[serde(default)]
    labour_high_gbp_pence: i64,
    labour_confidence_bps: Option<i64>,
    #[serde(default)]
    permit_traffic_mgmt_line_items: Vec<Value>,
    #[serde(default)]
    permit_traffic_mgmt_base_gbp_pence: i64,
    #[serde(default)]
    permit_traffic_mgmt_low_gbp_pence: i64,
    #[serde(default)]
    permit_traffic_mgmt_high_gbp_pence: i64,
    permit_responsibility: Option<String>,
    #[serde(default)]
    warnings: Vec<Stage11Issue>,
    #[serde(default)]
    blockers: Vec<Stage11Issue>,
}
```

### 5.2 LandedCostEstimateWrite Extensions

Add fields:

```rust
goods_landed_cost_gbp_pence: i64,
installation_labour_cost_gbp_pence: i64,
installation_labour_low_gbp_pence: i64,
installation_labour_high_gbp_pence: i64,
installation_labour_confidence_bps: Option<i64>,
permit_traffic_mgmt_cost_gbp_pence: i64,
permit_traffic_mgmt_low_gbp_pence: i64,
permit_traffic_mgmt_high_gbp_pence: i64,
permit_responsibility: Option<String>,
```

Persistence SQL must add columns, update assignment list, and parameter bindings.

### 5.3 Rollup Functions

Domestic:

```rust
let goods_landed_cost_gbp_pence = supplier_cost.supplier_cost_gbp_pence;
let labour_base = labour.map_or(0, |result| result.labour_base_gbp_pence);
let permit_base = labour.map_or(0, |result| result.permit_traffic_mgmt_base_gbp_pence);
let economic_landed_cost_gbp_pence = goods_landed_cost_gbp_pence
    .saturating_add(labour_base)
    .saturating_add(permit_base);
```

Import:

```rust
let goods_landed_cost_gbp_pence = supplier_cost
    .supplier_cost_gbp_pence
    .saturating_add(freight.base_gbp_pence)
    .saturating_add(duty_total)
    .saturating_add(base_import_vat);

let base_landed_cost = goods_landed_cost_gbp_pence
    .saturating_add(labour_base)
    .saturating_add(permit_base);
```

Low/high should use labour and permit low/high for net margin bands. `goods_landed_cost_gbp_pence` stays the ratio numerator and is persisted.

### 5.4 Live Loop Invocation

Load:

```rust
let labour_skill = load_skill(
    &repo_root.join("skills/landed-cost/uk-installation-labour-estimate"),
    "skills/landed-cost/uk-installation-labour-estimate/SKILL.md",
)?;
```

Invocation order:

- Domestic: supplier basis -> labour skill when installation requirement exists -> domestic rollup.
- Import: customs -> sizing -> SeaRates -> FX -> labour skill when installation requirement exists -> import rollup.

Add `uk_installation_labour` to:

- `skill_versions`.
- `skill_session_ids`.
- `statuses`.
- run counters in loop summary.
- confidence breakdown projection.

No `crates/daemon/src/sources/registry.rs` edit is needed.

### 5.5 Goods-Share Calculator

Use a pure integer function:

```rust
fn goods_value_share_bps(goods_landed: i64, labour: i64, permits: i64) -> i64 {
    let denominator = goods_landed
        .saturating_add(labour)
        .saturating_add(permits);
    if denominator <= 0 {
        return 0;
    }
    goods_landed.saturating_mul(10_000) / denominator
}
```

Decision synthesis must prefer an existing contract-value basis when present. The cost-proxy fallback is used when the tender does not itemise goods/labour.

Near-floor cost-proxy values route to review.

## 6. TypeScript Integration

### 6.1 contracts-schema

Extend:

- `ProductCandidateRow`: expose `InstallationRequirement` in `critical_attributes_json` typing or additive fields.
- `LandedCostEstimateRow`: add labour, permit/TM, goods-only subtotal.
- `PipelineAssessmentRow`: add goods-share and coverage fields.
- Add `MaintenancePeriodEstimateRow` or an equivalent first-class type for the recurring-maintenance table.

Keep `SupplierQuoteRow.quote_class` free-form. Tier B installer quotes use `installer_quote`.

### 6.2 coverage-rollup

Add pure helpers:

```typescript
export function computeGoodsValueShareBps(input: {
  goodsLandedCostGbpPence: number;
  installationLabourCostGbpPence: number;
  permitTrafficMgmtCostGbpPence: number;
}): number {
  const denominator =
    input.goodsLandedCostGbpPence +
    input.installationLabourCostGbpPence +
    input.permitTrafficMgmtCostGbpPence;
  if (denominator <= 0) return 0;
  return Math.round((input.goodsLandedCostGbpPence * 10_000) / denominator);
}
```

Extend `CoverageCandidateInput` or a sibling model to count labour/permit coverage separately from landed-cost coverage.

### 6.3 Tests

Add/extend:

- `goods-qualification.test.ts`: `TS-QUAL-003a/b`, `TS-QUAL-006`, `TS-QUAL-007`, `TS-QUAL-008`.
- `description-quality.test.ts`: eight-row truth table.
- `coverage-rollup.test.ts`: `TS-COV-007`, `TS-COV-008`, `TS-COV-009`.

## 7. UI Integration

### 7.1 Manifest Viewer

`packages/ui/manifest-viewer.tsx` should render:

| Row | Fields |
|---|---|
| Installation labour | low/base/high, confidence bps, region, scope, line-item summary |
| Access equipment | equipment type, operated flag, days, delivery, subtotal |
| Permits & traffic management | low/base/high, responsibility, warning flags |
| Goods value share | basis, numerator, denominator, floor, review band, verdict |

### 7.2 Contract Card

`packages/ui/contract-card-frame.tsx` should add:

- `labourCoverageBps`.
- Review reason chips for near-floor proxy, permit responsibility unknown, scope unresolved, no-list/bespoke.

## 8. Validation Architecture

### 8.1 E2E Integration Target

New file: `crates/daemon/tests/installation_labour_engine.rs`.

Use existing shared DB helpers. Each scenario:

1. Create test DB and assert schema epoch 10.
2. Seed source, notice, lot, product candidate, supplier quote.
3. Seed or run landed-cost labour result.
4. Run rollup and Phase-2 gate.
5. Assert persisted ratio reconstructability and scenario verdict.

### 8.2 Canary Corpus

Directory: `tests/fixtures/calibration/landed-cost/uk-installation-labour/`.

Required files:

- `canary.inputs.jsonl`
- `golden.assertions.yaml`
- `validator-rubric.yaml`
- `metadata.json`
- `manifest.md`

Cases:

- `lights-nw-2day-01`
- `lights-lon-2day-01`
- `cctv-yh-install-01`
- `furniture-lon-assembly-01`
- `region-unknown-01`
- `scope-unresolved-01`
- `rate-stale-01`

### 8.3 UI Verification

Use `/agent-browser` for rendered UI verification:

1. Launch desktop control panel.
2. Open Scenario 1 review-routed process.
3. Assert manifest viewer shows Installation labour row and Permits & traffic management row.
4. Assert contract card shows `labourCoverageBps`.

## 9. Implementation Sequence Reference

Detailed sequencing belongs to the strategy document, but the architecture expects these dependency edges:

1. Schema and contracts first.
2. TypeScript qualification/service-gate tests and changes.
3. Labour skill scaffold and reference tables.
4. Rust parser, live-loop invocation, rollup, persistence.
5. Goods-share Phase-2 decision synthesis.
6. Maintenance entity and annualised margin projection.
7. Overlay and refresh skill.
8. UI surfacing.
9. Full E2E/canary/gold-set gate.

Tier B research spikes are tracked after Tier A design surfaces exist and must not block Tier A shipping unless their spike changes a Tier A boundary.
