# Stage 11 MVP Landed Cost - Build, Validate, and Land Handoff

ULTRATHINK and be meticulous.

This is an implementation handoff for a fresh coding agent. It is **not** a research-only task. Your mission is to build the **MVP-grade Stage 11 landed-cost workflow** using the platform's existing skill/agent-runner architecture, validate it against live L1-L3 data, and loop until real `landed_cost_estimates` rows are saved in SQLite.

This handoff targets a simpler MVP than the prior decision-grade/L5 research:

- One customs source: **HMRC Trade Tariff**, reached through a skill-backed agent workflow because commodity classification and measure interpretation require judgment.
- One freight source: **SeaRates**, reached through a skill-backed browser workflow connected to the existing persistent CDP browser.
- Simple FX: use the easiest robust live conversion path, such as Google conversion lookup through a skill/browser flow when the source currency is not GBP. Do not build Xe/HMRC dual-FX infrastructure for MVP.
- Critical pre-step before SeaRates: derive shipment weight, dimensions, and cubic metres from the selected quote/listing/product quantity. If the selected supplier quote does not contain enough dimensional data, the workflow must revisit the source evidence/listing through a skill or persist an explicit blocker/warning instead of pretending precision.

The target is **L1-L3 MVP screening-grade landed cost**, not L5 decision-grade customs intelligence. Do not implement FPO, Freightos, Xe, CBAM, TRA daily watchers, multi-source recall, full measure-interaction engines, or broker-grade declaration logic unless absolutely required to make the MVP honest.

Fresh-eye audit note: this prompt is intentionally an implementation handoff, not another planning document. A receiving agent should build, run, validate, fix, and loop until the database contains real Stage 11 rows. If a component cannot complete because live data is missing, the correct MVP behavior is to persist a row with explicit blockers rather than skip the candidate.

---

## 0. Non-Negotiable Outcome

By the end of this task, the repository must support a real Stage 11 MVP run that:

1. Reads Stage 10 `candidate_pricing_assessments`.
2. Uses the selected `supplier_quotes`.
3. Runs skill-based HMRC customs classification/duty estimation.
4. Computes or skill-resolves shipment sizing: total units, total weight, total CBM, packing/container/LCL/FCL basis.
5. Runs skill-based SeaRates freight estimation through the persistent CDP browser.
6. Performs simple FX conversion only where needed.
7. Writes one `landed_cost_estimates` row per process/candidate/selected quote that Stage 11 attempts, including rows with blockers when a real estimate cannot be completed.
8. Marks Stage 11 stage heads/statuses consistently with existing stage tracking.
9. Produces a validation manifest proving the workflow ran on the current database.

The most important acceptance signal is:

```sql
SELECT COUNT(*) FROM landed_cost_estimates;
```

It must be greater than zero after a real Stage 11 MVP run. Rows with honest blockers are acceptable for incomplete data; silent skips are not.

### 0.1 Concrete Deliverables

Deliver these artifacts:

1. New or updated landed-cost skills:
   - `skills/landed-cost/hmrc-customs-classification/SKILL.md`
   - `skills/landed-cost/hmrc-customs-classification/result.schema.json`
   - `skills/landed-cost/shipment-sizing/SKILL.md`
   - `skills/landed-cost/shipment-sizing/result.schema.json`
   - `skills/landed-cost/searates-freight-estimate/SKILL.md`
   - `skills/landed-cost/searates-freight-estimate/result.schema.json`
   - optional `skills/landed-cost/simple-fx-lookup/*` if non-GBP freight requires a separate skill
2. Rust support for Stage 11 MVP orchestration, preferably via `cargo run -p daemon -- --ingest landed-cost`.
3. Rust parsers/validators for every new skill result contract.
4. Idempotent persistence into `landed_cost_estimates`.
5. Stage tracking rows for stage `"landed cost"`.
6. Tests/canaries for domestic row persistence, blocked import row persistence, landed-cost arithmetic, HMRC rebar parsing, and SeaRates output parsing.
7. Validation manifest at `docs/pipeline/stage-11-mvp-validation-manifest.md`.

Do not mark the work complete if only the skills exist. The stage must write database rows.

---

## 1. Read These First

Read these files before editing:

1. `AGENTS.md`
   - Follow all repository rules: no file deletion, work on `main`, Cargo for Rust, Bun for TS, no script-based code changes.
2. `docs/research/stage-11-landed-cost-research.md`
   - Treat this as the source-of-truth research baseline. It is intentionally broader than this MVP. Extract only the L1-L3 screening-grade parts.
3. `prompts/stage-11-landed-cost-research-handoff.md`
   - Historical research handoff. Use it for verified current-state facts and drift checks.
4. `prompts/stage-10-gross-margin-gate-handoff.md`
   - Stage 10 is the direct input contract for Stage 11.
5. `docs/pipeline/stage-10-validation-manifest.md`
   - Current DB and validation shape from the stage immediately before landed cost.
6. `crates/daemon/src/main.rs`
   - Study CLI dispatch, deterministic pipeline, skill execution, marketplace live-loop, Stage 10, fixture landed-cost writes.
7. `migrations/0001_sqlite_state_initial.sql`
   - Executable schema truth for `supplier_quotes`, `candidate_pricing_assessments`, `landed_cost_estimates`, `workflow_jobs`, and `process_stage_heads`.
8. `skills/marketplace-search/alibaba/SKILL.md`
   - Existing CDP skill pattern for a logged-in browser and result contracts.
9. `skills/marketplace-search/alibaba/result.schema.json`
   - Existing result-contract style.
10. `apps/desktop/src-tauri/src/query_service.rs`
    - Existing desktop VM reads a compact subset of landed-cost rows.

Do not assume the prior L5 plan is the build target. This handoff is the smaller, practical MVP build.

### 1.1 Understanding Checkpoints

Before editing, be able to answer these questions:

1. Why can Stage 11 not be implemented purely inside the current synchronous `run_deterministic_pipeline()`?
2. Which table/row selects the supplier quote that Stage 11 must cost?
3. Which `landed_cost_estimates` columns are NOT NULL and therefore must always be populated even for blocked rows?
4. What is the difference between a domestic UK benchmark quote and an import quote for MVP Stage 11?
5. Why is `7214200000` not enough for the steel rebar customs canary, and why must the HMRC skill inspect import measures beyond the basic duty summary?
6. What exact evidence does SeaRates need before it can produce a useful freight estimate?
7. How will `net_margin_low_bps`, `net_margin_base_bps`, and `net_margin_high_bps` be populated when Stage 12 is not yet implemented?

---

## 2. Current Verified State

The current repository state, verified before this handoff, is:

```text
candidate_pricing_assessments=4
landed_cost_estimates=0
supplier_quotes=10
non_gbp_quotes=0
```

Selected Stage 10 quote summary:

```text
Aggregates (general)
  selected quote: amazon-uk / uk_retail_benchmark
  unit_price_gbp_pence=88
  ship_from_country=GB
  quoted_quantity=17.5 kg
  revenue=16722222p
  gross_margin_bps=9999

Double-glazed unit (sealed insulating glass unit, BS EN 1279-compliant, argon-filled, warm-edge spacer)
  selected quote: alibaba-search / cn_supplier_floor
  unit_price_gbp_pence=516
  ship_from_country=CN
  quoted_quantity=10000 square meter
  minimum_order_quantity=10
  revenue=0
  gross_margin_bps=0

Ely Library carpet / flooring package
  selected quote: amazon-uk / uk_retail_benchmark
  unit_price_gbp_pence=6495
  ship_from_country=GB
  quoted_quantity=1 unit
  revenue=0
  gross_margin_bps=0

Library carpet / floor covering (Ely Library refresh)
  selected quote: amazon-uk / uk_retail_benchmark
  unit_price_gbp_pence=8093
  ship_from_country=GB
  quoted_quantity=1 unit
  revenue=0
  gross_margin_bps=0
```

Important data gaps:

```text
supplier_quotes.country_of_origin present: 0
supplier_quotes.country_of_dispatch present: 0
supplier_quotes.incoterm present: 0
supplier_quotes.lead_time_days present: 0
product_candidates.quantity present: 1/820
product_candidates.material present: 0
product_candidates.numeric_specs_json present: 2/820
```

This means the MVP must handle incomplete input gracefully. For the China Alibaba DGU row, Stage 11 will likely need to inspect evidence/listing context for dimensions and origin assumptions. For UK retail benchmark rows, landed-cost can be domestic/no-import with simple freight/final-mile assumptions or an explicit `domestic_quote_no_import` warning.

Always re-run the baseline SQLite queries before building. These counts are a verified snapshot, not permission to hard-code assumptions.

---

## 3. Architecture Reality Check

### 3.1 Current CLI Dispatch

`crates/daemon/src/main.rs` currently dispatches:

```rust
if let Some(skill_path) = &cli.skill_path {
    run_bound_skill(...).await?;
}

if let Some(source_id) = &cli.ingest {
    match source_id.as_str() {
        "fts" => run_fts_live_loop(...).await?,
        "contracts-finder" => run_contracts_finder_live_loop(...).await?,
        "procontract" => run_procontract_discover_live_loop(...).await?,
        "procontract-interest" => run_procontract_register_interest_live_loop(...).await?,
        "procontract-enrich" => run_procontract_enrich_tender_live_loop(...).await?,
        "gem-imports" | "mx-wholesale" | "amazon-uk" | "ebay-uk" | "alibaba-search" => {
            run_marketplace_search_live_loop(...).await?
        }
        other => bail!("unsupported source: {other}"),
    };
}

if let Some(source_id) = &cli.pipeline {
    run_deterministic_pipeline(&mut runtime, source_id)?;
}

if cli.run_next_job {
    run_next_workflow_job(&mut runtime, &cli).await?;
}
```

Implication: the existing `--pipeline` path is synchronous and currently stops after Stage 10 + relation resolution. Browser/skill work belongs in async live-loop or workflow-job paths.

### 3.2 Current Deterministic Pipeline

`run_deterministic_pipeline()` currently does:

```rust
run_extract_lots_stage(...)
run_prequalify_stage(...)
run_qualify_lot_stage(...)
run_reclassify_candidates_stage(...)
run_score_quoteability_stage(...)
run_gross_margin_gate_stage(...)
run_relate_stage(...)
```

It does **not** call Stage 11. Do not simply wedge blocking browser work into this synchronous function. The MVP should introduce a Stage 11 live-loop/agent-runner path and optionally have the deterministic pipeline report/enqueue Stage 11 work.

### 3.3 Existing Skill Execution Pattern

Study these existing patterns in `crates/daemon/src/main.rs`:

- `run_marketplace_search_live_loop()`:
  - Loads a skill.
  - Selects candidate payload.
  - Creates a `workflow_jobs` row.
  - Claims it.
  - Calls `execute_skill_session(...)`.
  - Reads `result.json`.
  - Persists domain rows.
  - Completes the workflow job.
- `run_extract_document_semantics_job()`:
  - Parses job request.
  - Loads a skill.
  - Calls `execute_skill_session(...)`.
  - Reads structured result.
  - Persists domain output.
  - Calls `runner.persist_success(...)`.
- `run_next_workflow_job()`:
  - Currently supports only extraction jobs. Add Stage 11 job kinds here if you choose a queued-job architecture.

MVP recommendation: implement `--ingest landed-cost` as an async live-loop that mirrors marketplace search and internally runs the Stage 11 skills. This avoids forcing long browser tasks through the synchronous pipeline while still producing durable DB rows.

### 3.4 Verified Code Location Map

Use this map to avoid wandering:

| Surface | File / current line | Why it matters |
|---|---:|---|
| CLI struct | `crates/daemon/src/main.rs:355` | `--ingest`, `--pipeline`, `--skill-path`, and `--run-next-job` are defined here. |
| CLI dispatch | `crates/daemon/src/main.rs:1095-1178` | Add the `"landed-cost"` ingest branch here if using the recommended live-loop trigger. |
| `LiveLoopReport` | `crates/daemon/src/main.rs:739` | Return the same summary/status shape as existing live loops. |
| `run_next_workflow_job` | `crates/daemon/src/main.rs:1288` | Extend only if you choose queued Stage 11 jobs. |
| Semantic skill job pattern | `crates/daemon/src/main.rs:1632` | Shows `execute_skill_session`, `result.json` read, domain persistence, and `persist_success`. |
| Marketplace persistence | `crates/daemon/src/main.rs:3758` | Shows how skill result JSON is turned into durable domain rows. |
| Existing static FX helper | `crates/daemon/src/main.rs:3742` | This is fallback-grade only for Stage 11; do not silently use static rates as live FX. |
| Marketplace candidate loader | `crates/daemon/src/main.rs:4056` | Useful pattern for assembling rich candidate/notice/lot context. |
| `execute_skill_session` | `crates/daemon/src/main.rs:4436` | Central skill runner entry point. |
| Marketplace live-loop | `crates/daemon/src/main.rs:6598` | Best local model for `run_landed_cost_live_loop`. |
| Deterministic pipeline | `crates/daemon/src/main.rs:9840` | Currently stops before Stage 11; do not place long browser work here. |
| Stage 10 writer | `crates/daemon/src/main.rs:11178` | Direct upstream contract for Stage 11. |
| `persist_stage` | `crates/daemon/src/main.rs:11446` | Writes accepted validation/status/head rows through `persist_stage_validation`. |
| `landed_cost_estimates` schema | `migrations/0001_sqlite_state_initial.sql:322` | Executable schema truth. |
| Stage tracking schema | `migrations/0001_sqlite_state_initial.sql:453` and `:477` | `process_stage_status` and `process_stage_heads`. |
| Desktop landed-cost VM | `apps/desktop/src-tauri/src/query_service.rs:508` and `:1188` | Desktop reads only id/candidate/quote/economic cost/base net margin/grade/confidence. |

Line numbers are a starting point. Re-run `rg` before editing because this repo moves quickly.

### 3.5 Skill Loading Contract

Every new Stage 11 skill must be loadable by `load_skill(skill_root, relative_skill_path)`. That function requires:

1. `SKILL.md` at the skill root.
2. YAML frontmatter in `SKILL.md` containing at least these fields:
   - `skill_id`
   - `skill_version`
   - `interaction_profile`
   - `result_schema_version`
   - `model_profile`
3. `result.schema.json` at the same skill root.

Current daemon support for `interaction_profile` is narrow:

```rust
fn skill_manifest(skill: &LoadedSkill) -> Result<SkillManifest> {
    let interaction_profile = match skill.metadata.interaction_profile.as_str() {
        "pty_interactive" => InteractionProfile::PtyRequired,
        "headless_contract" => InteractionProfile::ContractRequired,
        other => bail!("unsupported interaction_profile `{other}`"),
    };
    Ok(SkillManifest {
        interaction_profile,
    })
}
```

For these MVP skills, use `pty_interactive`. The HMRC skill may be API-heavy, but it still benefits from an agent session for classification judgment. SeaRates and Alibaba-backed shipment sizing require browser/CDP behavior.

Important persistence trap: `cargo run -p daemon -- --skill-path <path>` can run arbitrary skills, but the current manual persistence branch only auto-persists procurement, marketplace-search, and PDF extraction outputs. A new `landed-cost/*` skill will not automatically write `landed_cost_estimates` from `--skill-path`. Use `--skill-path` only for canary/debug runs unless you also add explicit landed-cost manual persistence. The real Stage 11 MVP persistence should happen in `run_landed_cost_live_loop()` or the queued Stage 11 jobs that read `result.json`, validate it, and assemble the landed-cost row.

Recommended frontmatter:

```yaml
---
skill_id: landed-cost/hmrc-customs-classification
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
maxPages: 8
maxScrollCycles: 2
---
```

```yaml
---
skill_id: landed-cost/shipment-sizing
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
requires_cdp: true
cdp_default_port: 9222
maxPages: 8
maxScrollCycles: 3
---
```

```yaml
---
skill_id: landed-cost/searates-freight-estimate
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
requires_cdp: true
cdp_default_port: 9222
maxPages: 8
maxScrollCycles: 3
---
```

For optional `landed-cost/simple-fx-lookup`, use the same shape with `requires_cdp: true` only if the chosen implementation uses browser/Google rather than a deterministic API/helper.

---

## 4. Data Contracts to Preserve

### 4.1 Stage 10 Input Table

`candidate_pricing_assessments` schema:

```sql
CREATE TABLE IF NOT EXISTS candidate_pricing_assessments (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  notice_id TEXT NOT NULL REFERENCES contract_notices(id),
  product_candidate_id TEXT NOT NULL REFERENCES product_candidates(id),
  selected_quote_id TEXT NOT NULL REFERENCES supplier_quotes(id),
  selected_quote_row_version INTEGER NOT NULL,
  quote_cost_boundary TEXT NOT NULL,
  revenue_basis_type TEXT NOT NULL,
  expected_addressable_revenue_ex_vat_pence INTEGER NOT NULL,
  revenue_confidence_bps INTEGER NOT NULL,
  gross_margin_bps INTEGER NOT NULL,
  gross_estimate_confidence_bps INTEGER NOT NULL,
  identity_gate_passed INTEGER NOT NULL,
  usability_gate_passed INTEGER NOT NULL,
  pricing_input_confidence_bps INTEGER NOT NULL,
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

Stage 11 reads this table. It does not reselect the quote.

### 4.2 Supplier Quote Fields Stage 11 Needs

`supplier_quotes` contains:

```sql
id TEXT PRIMARY KEY,
process_id TEXT NOT NULL,
notice_id TEXT NOT NULL,
product_candidate_id TEXT,
product_name TEXT NOT NULL,
source_name TEXT NOT NULL,
source_url TEXT,
source_listing_id TEXT,
ship_from_country TEXT,
country_of_origin TEXT,
country_of_dispatch TEXT,
incoterm TEXT,
quote_cost_boundary TEXT,
quoted_quantity NUMERIC,
quoted_quantity_unit TEXT,
unit_price_original_minor INTEGER NOT NULL,
currency TEXT NOT NULL,
unit_price_gbp_pence INTEGER NOT NULL,
offer_total_original_minor INTEGER,
offer_total_gbp_pence INTEGER,
price_basis_qty NUMERIC,
price_basis_unit TEXT,
minimum_order_quantity NUMERIC,
lead_time_days INTEGER,
includes_shipping INTEGER NOT NULL DEFAULT 0,
price_signal_role TEXT NOT NULL,
listing_pack_qty NUMERIC,
listing_pack_unit TEXT,
vat_inclusive INTEGER,
match_confidence_bps INTEGER,
quote_usability_confidence_bps INTEGER,
shipping_basis_confidence_bps INTEGER,
quote_realism_confidence_bps INTEGER,
evidence TEXT NOT NULL
```

Current Alibaba evidence stores `price_tiers` and `seller_identifier` but usually does **not** store unit weight/dimensions. Plan for that gap.

### 4.3 Stage 11 Output Table

`landed_cost_estimates` has many L5-ready nullable columns. MVP does not need to fill all of them, but it must populate all NOT NULL columns honestly:

```sql
CREATE TABLE IF NOT EXISTS landed_cost_estimates (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  notice_id TEXT NOT NULL REFERENCES contract_notices(id),
  product_candidate_id TEXT NOT NULL REFERENCES product_candidates(id),
  supplier_quote_id TEXT NOT NULL REFERENCES supplier_quotes(id),
  shipping_mode TEXT NOT NULL,
  mode_confidence_bps INTEGER NOT NULL,
  packing_scenario TEXT,
  declarable_commodity_code_10 TEXT,
  candidate_commodity_codes_json TEXT,
  additional_codes TEXT,
  additional_code_requirements_json TEXT,
  supplementary_units_required_json TEXT,
  commodity_confidence_bps INTEGER NOT NULL,
  classification_stability_bps INTEGER NOT NULL,
  customs_valuation_method TEXT,
  valuation_mode TEXT,
  customs_fx_rate NUMERIC NOT NULL,
  commercial_fx_rate NUMERIC NOT NULL,
  fx_basis_delta_bps INTEGER NOT NULL,
  fx_rate_checked_at TEXT,
  supplier_cost_gbp_pence INTEGER NOT NULL,
  freight_low_gbp_pence INTEGER NOT NULL,
  freight_base_gbp_pence INTEGER NOT NULL,
  freight_high_gbp_pence INTEGER NOT NULL,
  customs_duty_gbp_pence INTEGER NOT NULL,
  trade_remedy_duty_gbp_pence INTEGER,
  duty_outcome_min_gbp_pence INTEGER,
  duty_outcome_max_gbp_pence INTEGER,
  customs_cost_spread_bps INTEGER,
  trade_remedy_state TEXT,
  trade_remedy_cases_json TEXT,
  duty_breakdown_json TEXT,
  measure_interaction_json TEXT,
  exporter_rate_basis TEXT,
  exporter_identity_confidence_bps INTEGER,
  origin_evidence_state TEXT,
  import_vat_gbp_pence INTEGER NOT NULL,
  vat_rate_state TEXT,
  import_vat_treatment TEXT NOT NULL,
  vat_recovery_rate_bps INTEGER,
  uk_final_mile_cost_gbp_pence INTEGER NOT NULL,
  other_fees_gbp_pence INTEGER NOT NULL,
  economic_landed_cost_gbp_pence INTEGER NOT NULL,
  cash_outlay_gbp_pence INTEGER NOT NULL,
  cash_to_clear_gbp_pence INTEGER NOT NULL,
  working_capital_cost_gbp_pence INTEGER,
  revenue_basis_type TEXT NOT NULL,
  revenue_confidence_bps INTEGER NOT NULL,
  customs_resolution_confidence_bps INTEGER,
  commercial_estimate_confidence_bps INTEGER,
  net_margin_low_bps INTEGER NOT NULL,
  net_margin_base_bps INTEGER NOT NULL,
  net_margin_high_bps INTEGER NOT NULL,
  decision_grade TEXT NOT NULL,
  confidence_bps INTEGER NOT NULL,
  confidence_breakdown TEXT,
  warnings TEXT NOT NULL,
  blockers TEXT NOT NULL DEFAULT '[]',
  tariff_checked_at TEXT,
  measures_fresh_until TEXT,
  classification_fresh_until TEXT,
  trade_remedy_watch_state TEXT,
  cbam_exposure_flag INTEGER,
  cbam_estimate_gbp_pence INTEGER,
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

Suggested MVP row vocabulary:

- `decision_grade`: use `"review"` for screening-grade rows, `"blocked"` only if a required component cannot be estimated, and document the vocabulary in the manifest. Avoid fixture-only `"reviewable"`.
- `valuation_mode`: use `"mvp_proxy"` unless an actual invoice/declaration basis exists.
- `customs_valuation_method`: use `"transaction_value_proxy"` for selected quote value.
- `import_vat_treatment`: use `"recoverability_unknown"` unless the buyer/accounting treatment is known.
- `warnings`: JSON array string. Prefer arrays of objects with `code`, `message`, and `severity`.
- `blockers`: JSON array string. Prefer arrays of objects with `code`, `message`, and `severity`.

### 4.4 Minimum Row Population Rules

Blocked rows still have to satisfy the executable schema. Use this matrix when assembling `landed_cost_estimates` so every attempted assessment can persist a valid row.

| Column group | Domestic UK quote | Import complete | Import blocked |
|---|---|---|---|
| Identity FKs | Copy from Stage 10 assessment and selected quote. | Copy from Stage 10 assessment and selected quote. | Copy from Stage 10 assessment and selected quote. |
| `shipping_mode` | `"domestic"` | `"sea_freight"` / `"lcl"` / `"fcl"` from sizing/freight. | Best known mode, usually `"unknown_import"` or sizing output. |
| `mode_confidence_bps` | 5000-7000 depending on evidence. | From sizing/freight confidence. | Low confidence such as 1000-3000. |
| `commodity_confidence_bps` | 10000 only if no import classification is required; otherwise 0 with warning. | HMRC skill confidence. | 0 unless HMRC produced a candidate but blocked later. |
| `classification_stability_bps` | 10000 for no-import domestic path. | HMRC skill stability. | 0 unless HMRC produced a candidate set. |
| `customs_fx_rate` / `commercial_fx_rate` | 1.0 / 1.0. | 1.0 when all source amounts are GBP; live FX result otherwise. | 1.0 when no conversion happened; live FX result if available. |
| `fx_basis_delta_bps` | 0. | 0 for MVP unless two different rates are intentionally compared. | 0. |
| `supplier_cost_gbp_pence` | Computed selected-quote cost basis. | Computed selected-quote cost basis. | Computed selected-quote cost basis, or unit price with warning if quantity unknown. |
| `freight_low/base/high_gbp_pence` | 0 or domestic final-mile estimate. | SeaRates low/base/high converted to GBP. | 0 only with freight blocker, or conservative fallback only if explicitly warned. |
| `customs_duty_gbp_pence` | 0. | HMRC basic duty amount. | 0 if customs blocked before duty; partial amount if HMRC produced it. |
| `trade_remedy_duty_gbp_pence` | 0. | HMRC safeguard/AD/CVD amount if applicable, else 0. | 0 or partial amount if discovered. |
| `import_vat_gbp_pence` | 0 unless deliberately modeling VAT on domestic quote. | Computed VAT amount. | 0 if required inputs are missing; include blocker/warning. |
| `uk_final_mile_cost_gbp_pence` | 0 or domestic delivery estimate. | Included if SeaRates gives door delivery; otherwise 0 with warning. | 0 with warning if unknown. |
| `other_fees_gbp_pence` | 0 unless known. | MVP conservative clearance/broker fee if chosen, else 0 with warning. | 0 unless known. |
| `economic_landed_cost_gbp_pence` | Supplier cost + domestic freight/final mile + chosen VAT policy. | Supplier + freight base + duties + VAT policy + other/final mile. | Sum of known components only; blocker explains missing component. |
| `cash_outlay_gbp_pence` | At least supplier cost plus modeled delivery/VAT if included. | Supplier + freight + duty + remedy + VAT + fees. | Sum of known cash components; blocker explains missing component. |
| `cash_to_clear_gbp_pence` | 0 for domestic no-import path unless VAT modeled. | Duty + remedy + import VAT + clearance fees. | Known customs cash only, often 0 with blocker. |
| `revenue_basis_type` / `revenue_confidence_bps` | Copy from Stage 10. | Copy from Stage 10. | Copy from Stage 10. |
| `net_margin_low/base/high_bps` | Compute from Stage 10 revenue, or 0 for RFQ-required revenue. | Compute from low/base/high landed costs. | 0 if revenue missing; otherwise compute from known components and keep blockers. |
| `decision_grade` | `"review"`. | `"review"` for MVP screening-grade. | `"blocked"`. |
| `confidence_bps` | Minimum of pricing, shipping, and domestic confidence. | Minimum/weighted minimum of customs, sizing, freight, FX, pricing. | Low confidence such as 0-3000. |
| `warnings` | JSON array, may include `DOMESTIC_QUOTE_NO_IMPORT_CUSTOMS`. | JSON array of assumptions. | JSON array of partial assumptions. |
| `blockers` | Usually `[]`. | Usually `[]`; if not empty, grade should be `"blocked"`. | Non-empty array with machine-readable codes. |

Do not use SQL `NULL` for NOT NULL fields to signal uncertainty. Use numeric zero plus a blocker/warning when the field cannot be honestly estimated.

### 4.5 Provenance to Store in `confidence_breakdown`

The `landed_cost_estimates` schema does not have a dedicated `selected_quote_row_version` column, but Stage 10 records `candidate_pricing_assessments.selected_quote_row_version` for staleness detection. Store Stage 11 provenance in `confidence_breakdown` as JSON so reruns can detect drift.

Minimum recommended shape:

```json
{
  "schema": "stage11_mvp_confidence_breakdown_v1",
  "pricing_assessment_id": "pricing-...",
  "selected_quote_id": "sq-...",
  "selected_quote_row_version": 1,
  "supplier_cost_basis": {
    "basis": "quoted_quantity|minimum_order_quantity|unit_price_only|offer_total",
    "quantity": 10000,
    "quantity_unit": "square meter"
  },
  "skill_versions": {
    "hmrc_customs": "1.0.0",
    "shipment_sizing": "1.0.0",
    "searates_freight": "1.0.0",
    "simple_fx": null
  },
  "skill_session_ids": {
    "hmrc_customs": "session-...",
    "shipment_sizing": "session-...",
    "searates_freight": "session-..."
  },
  "customs_status": "classified|blocked|domestic_no_import",
  "shipment_sizing_status": "sized|blocked|not_required",
  "freight_status": "quoted|blocked|not_required",
  "fx_status": "not_required|converted|blocked",
  "vat_policy": "include_unknown_recovery_in_economic_cost|cash_only_unknown_recovery",
  "checked_at": "2026-04-28T00:00:00Z"
}
```

The exact fields can grow, but do not omit `selected_quote_row_version`, skill versions, statuses, and timestamp. Without them the next rerun cannot tell whether a landed-cost row is stale.

---

## 5. MVP Cost Equation

For this MVP, compute:

```text
supplier_cost_gbp_pence
  = selected_quote.unit_price_gbp_pence
    * shipment_basis_quantity_if_known
```

If `offer_total_gbp_pence` exists, prefer it. Otherwise multiply unit price by the best known quantity basis:

1. Contract/product candidate quantity if specific and compatible with quote unit.
2. Selected quote `quoted_quantity` if it represents the selected tier/order basis.
3. `minimum_order_quantity` if quote quantity is missing.
4. Unit price only, with warning `"quantity_unknown_used_unit_price"`.

Then:

```text
customs_value_gbp_pence = supplier_cost_gbp_pence + insurance_or_packing_if_known

customs_duty_gbp_pence =
  basic duty from HMRC measures against customs_value_gbp_pence

trade_remedy_duty_gbp_pence =
  safeguard / anti-dumping / countervailing additional duties if HMRC measures show them

import_vat_gbp_pence =
  vat_rate * (customs_value + customs_duty + trade_remedy_duty + freight + other import charges)

economic_landed_cost_gbp_pence =
  supplier_cost
  + freight_base
  + customs_duty
  + trade_remedy_duty
  + nonrecoverable_import_vat_component
  + uk_final_mile_cost
  + other_fees
  + working_capital_cost_if_used
```

### 5.1 Required Net-Margin Columns

`landed_cost_estimates` has NOT NULL net-margin columns, and the desktop VM already reads `net_margin_base_bps`. Stage 12 is not built yet, so Stage 11 MVP must populate these fields itself from Stage 10 revenue and the landed-cost scenarios.

Use this formula:

```text
net_margin_low_bps =
  if expected_addressable_revenue_ex_vat_pence <= 0:
    0
  else:
    (revenue - high_landed_cost) * 10000 / revenue

net_margin_base_bps =
  if expected_addressable_revenue_ex_vat_pence <= 0:
    0
  else:
    (revenue - base_landed_cost) * 10000 / revenue

net_margin_high_bps =
  if expected_addressable_revenue_ex_vat_pence <= 0:
    0
  else:
    (revenue - low_landed_cost) * 10000 / revenue
```

Scenario mapping:

- `low_landed_cost` uses `freight_low_gbp_pence` and the lowest defensible duty/freight assumptions.
- `base_landed_cost` uses `freight_base_gbp_pence`.
- `high_landed_cost` uses `freight_high_gbp_pence` and conservative blockers/warnings.

If Stage 10 revenue is zero or `revenue_basis_type = "rfq_required"`, persist all three net-margin fields as `0` and add a warning object with code `REVENUE_BASIS_RFQ_REQUIRED`. Do not divide by zero, and do not invent revenue to make net margin look meaningful.

Conservative MVP VAT handling:

- Calculate import VAT amount and store it.
- If recovery is unknown, do not silently exclude it from cash metrics.
- For `economic_landed_cost_gbp_pence`, decide and document one policy:
  - Conservative option: include VAT when recoverability is unknown.
  - Accounting option: exclude VAT from economic cost but include in `cash_to_clear_gbp_pence`.
- Whatever you choose, make it explicit in `warnings`, `confidence_breakdown`, and the validation manifest.

---

## 6. Skill 1: HMRC Customs Classification and Duty

Create a new skill directory, recommended:

```text
skills/landed-cost/hmrc-customs-classification/
  SKILL.md
  result.schema.json
```

This skill is allowed to use the HMRC Trade Tariff public API and browser/API investigation. It should not mutate external systems.

### 6.1 HMRC Endpoint Facts Already Tested

Do not use this as the root endpoint:

```text
https://www.trade-tariff.service.gov.uk/uk/api
```

It returned 404 with `{"error":"Resource not found"}` when tested.

Commodity routes do work:

```text
https://www.trade-tariff.service.gov.uk/uk/api/commodities/{commodity_code}
```

Steel rebar test finding:

```text
Parent/non-declarable code: 7214200000
  commodity endpoint returned 404 / non-declarable behavior

Declarable concrete reinforcement bars code: 7214200020
  endpoint: https://www.trade-tariff.service.gov.uk/uk/api/commodities/7214200020
  result: 200 JSON
  basic third-country duty: 0.00%
  VAT: 20%
  additional safeguard measure observed: 25% for relevant safeguard country group
```

Critical lesson: **do not read only the basic import trade summary**. The customs skill must inspect import measures enough to catch additional duties such as safeguards. For China-origin steel rebar, treating the result as "only 20% VAT" would be wrong if a safeguard measure applies.

### 6.2 Customs Skill Input

Pass a single candidate/quote context object per run:

```json
{
  "process_id": "...",
  "notice_id": "...",
  "product_candidate_id": "...",
  "pricing_assessment_id": "...",
  "selected_quote_id": "...",
  "candidate": {
    "name": "...",
    "head_noun": "...",
    "specification_text": "...",
    "standards": [],
    "critical_attributes": null,
    "numeric_specs": null,
    "material": null,
    "quantity": null,
    "quantity_unit": null,
    "lot_title": "...",
    "lot_description": "...",
    "notice_title": "...",
    "notice_summary": "...",
    "buyer_name": "...",
    "cpv_codes": "..."
  },
  "quote": {
    "source_name": "alibaba-search",
    "product_name": "...",
    "source_url": "...",
    "ship_from_country": "CN",
    "country_of_origin": null,
    "country_of_dispatch": null,
    "incoterm": null,
    "quoted_quantity": 10000,
    "quoted_quantity_unit": "square meter",
    "unit_price_gbp_pence": 516,
    "currency": "GBP",
    "unit_price_original_minor": 516,
    "evidence": "{...}"
  },
  "customs_value_gbp_pence": 51600000,
  "destination_country": "GB"
}
```

### 6.3 Customs Skill Output

`result.json` should include:

```json
{
  "source": "hmrc-trade-tariff",
  "product_candidate_id": "...",
  "selected_quote_id": "...",
  "classification_status": "classified",
  "declarable_commodity_code_10": "7214200020",
  "commodity_description": "Concrete reinforcement bars and rods excluding long products, such as round bars",
  "candidate_commodity_codes": [
    {
      "code": "7214200020",
      "description": "...",
      "declarable": true,
      "source_url": "https://www.trade-tariff.service.gov.uk/uk/api/commodities/7214200020",
      "reasoning": "..."
    }
  ],
  "commodity_confidence_bps": 7000,
  "classification_stability_bps": 6500,
  "origin_country": "CN",
  "origin_evidence_state": "quote_ship_from_country",
  "basic_duty_rate_bps": 0,
  "vat_rate_bps": 2000,
  "customs_duty_gbp_pence": 0,
  "trade_remedy_duty_gbp_pence": 12900000,
  "trade_remedy_state": "possible_or_applied",
  "trade_remedy_cases": [],
  "duty_breakdown": [
    {
      "measure_type": "third_country_duty",
      "rate_type": "ad_valorem",
      "rate_bps": 0,
      "amount_gbp_pence": 0,
      "source": "hmrc"
    },
    {
      "measure_type": "safeguard_additional_duty",
      "rate_type": "ad_valorem",
      "rate_bps": 2500,
      "amount_gbp_pence": 12900000,
      "source": "hmrc",
      "applicability": "requires_origin_group_check"
    }
  ],
  "additional_codes": null,
  "supplementary_units_required": [],
  "tariff_checked_at": "2026-04-28T00:00:00Z",
  "warnings": [
    {
      "code": "ORIGIN_INFERRED_FROM_SHIP_FROM",
      "message": "Origin inferred from ship_from_country because country_of_origin is missing.",
      "severity": "warning"
    }
  ],
  "blockers": []
}
```

If classification is uncertain, output:

```json
{
  "classification_status": "blocked",
  "declarable_commodity_code_10": null,
  "warnings": [],
  "blockers": [
    {
      "code": "CUSTOMS_CLASSIFICATION_UNCERTAIN",
      "message": "..."
    }
  ]
}
```

Do not fake a commodity code to keep the pipeline green.

### 6.4 Customs Skill Rules

- Prefer declarable 10-digit commodity codes.
- Reject or downgrade non-declarable headings/subheadings.
- Record all candidate codes considered.
- Preserve duty expression details in JSON. Do not flatten compound/specific duties into a single percent unless HMRC already expresses it as ad valorem and the unit basis is known.
- If supplementary units are required and the product data lacks them, return a blocker.
- Use `ship_from_country` only as an origin proxy and warn when `country_of_origin` is missing.
- For domestic GB-origin or GB-dispatched quotes, return a domestic/no-import classification path with zero customs duty and a warning that the selected quote is a UK retail benchmark, not an import transaction.

---

## 7. Shipment Sizing Pre-Step

This is essential. SeaRates estimates are only useful if the workflow knows:

- total shipment weight in kg,
- total shipment volume in cubic metres,
- origin,
- destination,
- date,
- whether the shipment is LCL, 20ft, 40ft, or multiple containers.

Current Stage 9 quotes do not reliably store dimensions/weight. You must decide where to implement the MVP sizing step:

Recommended:

```text
skills/landed-cost/shipment-sizing/
  SKILL.md
  result.schema.json
```

This skill can inspect:

- selected supplier quote evidence,
- Alibaba source URL if present,
- product specification text,
- quote quantity/unit,
- contract candidate quantity,
- listing pack fields,
- visible listing details.

If the selected quote is from Alibaba and has a `source_url`, attach to the persistent CDP browser exactly like `skills/marketplace-search/alibaba/SKILL.md` and inspect the listing for:

- product dimensions,
- package dimensions,
- unit weight,
- package weight,
- MOQ and packing unit,
- shipping terms,
- lead time.

### 7.1 Shipment Sizing Output

`result.json` should include:

```json
{
  "product_candidate_id": "...",
  "selected_quote_id": "...",
  "sizing_status": "sized",
  "units_required": 10000,
  "unit_basis": "square_meter",
  "unit_weight_kg": 12.5,
  "unit_volume_cbm": 0.04,
  "total_weight_kg": 125000,
  "total_volume_cbm": 400,
  "package_count": 120,
  "container_plan": {
    "mode": "fcl",
    "containers": [
      { "type": "40ft", "count": 7 }
    ],
    "load_factor_assumption": 0.85,
    "reasoning": "..."
  },
  "evidence": [
    {
      "source_url": "...",
      "raw_text": "...",
      "field": "package_weight"
    }
  ],
  "confidence_bps": 6500,
  "warnings": [],
  "blockers": []
}
```

If dimensions/weight cannot be found:

```json
{
  "sizing_status": "blocked",
  "total_weight_kg": null,
  "total_volume_cbm": null,
  "warnings": [],
  "blockers": [
    {
      "code": "SHIPMENT_DIMENSIONS_MISSING",
      "message": "No reliable unit/package dimensions or weight found on quote/listing."
    }
  ]
}
```

### 7.2 Container Heuristics

Use conservative published/common logistics assumptions as heuristics, and record them as assumptions:

- LCL if the shipment is clearly smaller than a container and SeaRates supports LCL for the route.
- 20ft container as the smallest FCL option.
- 40ft container for larger volume when economical.
- Use practical load factor, not theoretical maximum, unless SeaRates itself asks only for CBM/weight.
- If weight exceeds container payload before volume is full, split by weight.

Do not hard-code these as legal/logistics truth. They are MVP sizing assumptions and must appear in `warnings` or `confidence_breakdown`.

---

## 8. Skill 2: SeaRates Freight Estimate

Create a new skill directory, recommended:

```text
skills/landed-cost/searates-freight-estimate/
  SKILL.md
  result.schema.json
```

This is a CDP/browser skill. It must connect to the persistent browser session, not spawn a clean browser.

Use the same CDP discipline as the Alibaba skill:

```bash
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
agent-browser --cdp ${CDP_PORT} open "https://www.searates.com/"
```

Do not log out, alter account settings, contact carriers, book shipments, or create orders. This is quote lookup only.

If CDP is unreachable, do not launch a fresh anonymous browser and do not silently fall back to a made-up freight number. The skill should emit a structured blocker such as:

```json
{
  "freight_status": "blocked",
  "warnings": [],
  "blockers": [
    {
      "code": "CDP_CONNECT_FAILED",
      "message": "Persistent browser CDP endpoint was not reachable on the configured port.",
      "severity": "blocked"
    }
  ]
}
```

The Stage 11 live-loop should persist a blocked landed-cost row from that result. That row is valid MVP output because it tells the operator exactly what setup is missing.

### 8.1 SeaRates Input

```json
{
  "product_candidate_id": "...",
  "selected_quote_id": "...",
  "origin": {
    "country": "CN",
    "city_or_port": "Shanghai",
    "basis": "inferred_default_from_ship_from_country"
  },
  "destination": {
    "country": "GB",
    "city_or_postcode_or_port": "buyer location or UK default",
    "basis": "contract_buyer_location"
  },
  "ship_date": "2026-06-01",
  "shipment": {
    "total_weight_kg": 125000,
    "total_volume_cbm": 400,
    "container_plan": {
      "mode": "fcl",
      "containers": [
        { "type": "40ft", "count": 7 }
      ]
    }
  }
}
```

Origin/destination rules:

- If quote has a specific supplier city/port, use it.
- If only `ship_from_country = CN`, infer a conservative major port such as Shanghai/Shenzhen/Ningbo and record a warning.
- For UK destination, prefer contract notice delivery/buyer location if available. If missing, use a documented UK default and record a warning.
- Ship date should correspond to expected contract award/delivery if available; otherwise use a reasonable near-future date and record the basis.

### 8.2 SeaRates Browser Workflow

The skill should:

1. Open SeaRates.
2. Input origin and destination.
3. Input ship date.
4. Input shipment weight and CBM.
5. Select LCL/FCL/container type/count as appropriate.
6. Search for freight estimates.
7. Capture:
   - price,
   - currency,
   - route,
   - transit time if shown,
   - mode/carrier if shown,
   - whether quote is expired,
   - screenshot or raw evidence text,
   - caveats.

If SeaRates shows an expired estimate, **record the price anyway** and set `expired = true`. This MVP treats expired prices as useful reference evidence, but the row must carry an explicit warning.

### 8.3 SeaRates Output

```json
{
  "source": "searates",
  "product_candidate_id": "...",
  "selected_quote_id": "...",
  "freight_status": "quoted",
  "freight_currency": "USD",
  "freight_amount_minor": 125000,
  "freight_low_amount_minor": 110000,
  "freight_base_amount_minor": 125000,
  "freight_high_amount_minor": 150000,
  "route": {
    "origin": "Shanghai, CN",
    "destination": "Bristol, GB",
    "mode": "sea_freight",
    "container_basis": "7 x 40ft"
  },
  "transit_time_days": 35,
  "quote_expired": false,
  "source_url": "https://www.searates.com/...",
  "evidence": [
    {
      "kind": "raw_text",
      "value": "..."
    }
  ],
  "warnings": [],
  "blockers": []
}
```

If SeaRates cannot return a price:

```json
{
  "freight_status": "blocked",
  "warnings": [],
  "blockers": [
    {
      "code": "SEARATES_NO_QUOTE",
      "message": "..."
    }
  ]
}
```

---

## 9. Simple FX MVP

Current DB has `non_gbp_quotes=0`, so the first validation may not need FX conversion for supplier prices. SeaRates may return USD/EUR/GBP, so freight may need conversion.

Do not build the L5 dual FX engine. Do not add Xe. Do not add HMRC monthly FX unless you decide it is the simplest robust fallback.

MVP options:

1. If source amount is GBP, rate = `1.0`.
2. If source amount is non-GBP, run a simple FX lookup skill or deterministic helper:
   - Google query like `"1250 USD to GBP"`, through browser/skill if needed.
   - Parse rate and timestamp.
   - Persist the source in `confidence_breakdown`.
3. If live FX lookup fails, block or warn. Do not use stale magic constants silently.

Existing `convert_to_gbp_pence()` currently uses static USD/EUR approximations:

```rust
fn convert_to_gbp_pence(raw_minor: i64, currency: &str) -> i64 {
    match currency {
        "USD" => (raw_minor.saturating_mul(79)) / 100,
        "EUR" => (raw_minor.saturating_mul(85)) / 100,
        _ => raw_minor,
    }
}
```

That helper is acceptable for earlier marketplace rough pricing but is **not enough** for Stage 11 MVP landed-cost validation unless you explicitly document it as fallback-only.

Recommended:

```text
skills/landed-cost/simple-fx-lookup/
  SKILL.md
  result.schema.json
```

or a small Rust helper that accepts rates supplied by SeaRates/Google skill output. Keep it boring.

---

## 10. Rust Implementation Shape

You may choose the exact internal names, but keep the architecture legible.

Recommended additions in `crates/daemon/src/main.rs` unless the file is already being split:

```rust
async fn run_landed_cost_live_loop(
    runtime: &mut RuntimeBootstrap,
    cli: &Cli,
) -> Result<LiveLoopReport>
```

Add CLI dispatch:

```rust
"landed-cost" => run_landed_cost_live_loop(&mut runtime, &cli).await?
```

Selection query shape. Do not use `SELECT *` in the final Rust implementation because rusqlite row indices become fragile; list the columns you map into structs. This sketch shows the joins and filters that matter:

```sql
SELECT
  cpa.id,
  cpa.process_id,
  cpa.notice_id,
  cpa.product_candidate_id,
  cpa.selected_quote_id,
  cpa.expected_addressable_revenue_ex_vat_pence,
  cpa.revenue_basis_type,
  cpa.revenue_confidence_bps,
  cpa.gross_margin_bps,
  cpa.gross_estimate_confidence_bps,
  sq.id AS quote_id,
  sq.row_version AS quote_row_version,
  sq.product_name AS quote_product_name,
  sq.source_name,
  sq.source_url,
  sq.source_listing_id,
  sq.ship_from_country,
  sq.country_of_origin,
  sq.country_of_dispatch,
  sq.incoterm,
  sq.quoted_quantity,
  sq.quoted_quantity_unit,
  sq.unit_price_original_minor,
  sq.currency,
  sq.unit_price_gbp_pence,
  sq.offer_total_gbp_pence,
  sq.minimum_order_quantity,
  sq.lead_time_days,
  sq.includes_shipping,
  sq.price_signal_role,
  sq.listing_pack_qty,
  sq.listing_pack_unit,
  sq.vat_inclusive,
  sq.evidence AS quote_evidence,
  pc.name AS candidate_name,
  pc.head_noun,
  pc.specification_text,
  pc.standards_json,
  pc.critical_attributes_json,
  pc.numeric_specs_json,
  pc.material,
  pc.quantity,
  pc.quantity_unit,
  nl.title,
  nl.description,
  cn.title,
  cn.summary,
  cn.buyer_name,
  cn.main_category
FROM candidate_pricing_assessments cpa
JOIN supplier_quotes sq ON sq.id = cpa.selected_quote_id
JOIN product_candidates pc ON pc.id = cpa.product_candidate_id
JOIN contract_notices cn ON cn.id = cpa.notice_id
LEFT JOIN notice_lots nl ON nl.id = pc.lot_id
WHERE NOT EXISTS (
  SELECT 1 FROM landed_cost_estimates lce
  WHERE lce.product_candidate_id = cpa.product_candidate_id
    AND lce.supplier_quote_id = cpa.selected_quote_id
)
ORDER BY cpa.created_at, cpa.id
LIMIT ?;
```

This missing-row query is the first slice. A production-ready MVP should also select stale rows using the provenance stored in `confidence_breakdown`; see the rerun rules below.

The live-loop should:

1. Load candidates needing landed cost.
2. For each assessment:
   - Compute preliminary supplier cost basis.
   - If domestic UK quote, create a domestic landed-cost row without HMRC/SeaRates unless you choose to estimate UK final-mile.
   - If import quote, run HMRC customs skill.
   - Run shipment sizing step.
   - If shipment sizing succeeds, run SeaRates freight skill.
   - Convert freight to GBP if needed.
   - Assemble landed-cost row.
   - Persist row and stage status/head.
3. Continue processing other candidates if one candidate blocks.
4. Return summary counts:
   - attempted,
   - completed,
   - blocked,
   - domestic,
   - import,
   - customs_skill_runs,
   - searates_skill_runs.

### 10.1 Queued Job Alternative

If you choose a queued-job design, add job kinds to `run_next_workflow_job()`:

```rust
"landed_cost_customs_classification" => run_landed_cost_customs_job(...).await
"landed_cost_shipment_sizing" => run_landed_cost_sizing_job(...).await
"landed_cost_freight_estimate" => run_landed_cost_freight_job(...).await
"landed_cost_assemble" => run_landed_cost_assemble_job(...).await
```

This is more durable but bigger. For MVP, direct skill calls inside `run_landed_cost_live_loop()` are acceptable if artifacts and result contracts are persisted like marketplace-search.

### 10.2 Persistence Rules

Use a single write transaction for each final landed-cost row.

Use deterministic IDs:

```text
landed-{product_candidate_id}-{selected_quote_id}
```

or a stable hash of process/candidate/quote/stage. Avoid timestamp-only IDs that break idempotency.

Use `ON CONFLICT(id) DO UPDATE` and increment `row_version`.

Call `persist_stage(...)` with:

```text
stage = "landed cost"
scope_kind = "candidate_quote"
scope_id = "{product_candidate_id}::{selected_quote_id}"
```

Keep this stage name/scope stable and document it in the manifest.

### 10.2A Idempotency and Rerun Rules

Stage 11 must be safe to run repeatedly.

Use a stable row id:

```text
landed-{product_candidate_id}-{selected_quote_id}
```

Then `ON CONFLICT(id) DO UPDATE` should update all computed cost, confidence, warning, blocker, freshness, and `stage_run_id` fields and increment `row_version`.

Rerun a landed-cost row when any of these are true:

1. No row exists for `(product_candidate_id, selected_quote_id)`.
2. Stage 10 now points to a different `selected_quote_id`; this naturally creates a new stable id.
3. `candidate_pricing_assessments.selected_quote_row_version` differs from `confidence_breakdown.selected_quote_row_version`.
4. `supplier_quotes.row_version` differs from the version recorded in `confidence_breakdown`.
5. A prior row has non-empty `blockers` and the relevant missing input may now be available, such as CDP restored, dimensions now visible, or SeaRates quote now available.
6. `fx_rate_checked_at`, `tariff_checked_at`, `classification_fresh_until`, or `measures_fresh_until` is stale according to the MVP policy documented in the validation manifest.

Do not skip all existing rows forever. The simplest first build can process missing rows, but the validation manifest must say whether stale-row reruns are implemented or deferred. If deferred, create a follow-up issue in Beads and keep the stale-detection query in the manifest.

### 10.2B Stage Validation State for Blocked Rows

There is a difference between a **blocked estimate** and a **failed stage execution**.

- If Stage 11 successfully examined a candidate and persisted a `landed_cost_estimates` row whose `blockers` explain missing dimensions, no SeaRates quote, uncertain customs code, or missing FX, the stage execution should be considered accepted/succeeded. Use `persist_stage(...)` or an accepted `ValidationOutcome` so `process_stage_heads` advances.
- If the Rust code panics, the skill result contract is invalid, the database write fails, or the live-loop cannot parse its inputs, that is a failed execution and should not advance the head as a valid Stage 11 result.

This matters because the MVP's job is to make missing landed-cost evidence visible in the database. A blocked row is a successful visibility outcome, not a crash.

### 10.3 Domestic Quote Handling

For selected quotes with `ship_from_country = "GB"` and no import evidence:

- `customs_duty_gbp_pence = 0`
- `trade_remedy_duty_gbp_pence = 0`
- `import_vat_gbp_pence = 0` unless you explicitly model VAT on quote price
- `shipping_mode = "domestic"`
- `freight_base_gbp_pence = 0` or conservative final-mile estimate if available
- `warnings` must include an object with code `"DOMESTIC_QUOTE_NO_IMPORT_CUSTOMS"`
- `decision_grade = "review"`

This lets Stage 12/desktop coverage progress without pretending HMRC classification is needed for Amazon UK retail benchmark rows.

### 10.4 Import Quote Handling

For selected quotes with `ship_from_country != "GB"` or source `alibaba-search`:

- Run customs.
- Run sizing.
- Run freight if sizing succeeds.
- If customs blocks but freight succeeds, still persist row with cost components available and blockers.
- If sizing/freight blocks, still persist row with customs result and freight `0` or fallback only if clearly marked. Prefer blockers over fake freight.

---

## 11. Required Skill Result Schemas

Every new skill must have a `result.schema.json`, and the daemon must validate enough of the result before persistence to avoid panics or nonsense rows.

The existing `validate_result_schema` helper in `crates/daemon/src/agents/contracts.rs` checks top-level required fields and simple top-level types. Treat it as a first pass, not complete semantic validation. Stage 11 should also manually validate domain invariants before writing a row:

- status fields are one of the documented values;
- basis-point fields are between 0 and 10000;
- money fields are non-negative integers;
- `blockers` is non-empty when status is `"blocked"`;
- `declarable_commodity_code_10` is present when customs status is `"classified"`;
- SeaRates quoted output has a currency and base amount;
- shipment sizing output has positive `total_weight_kg` and `total_volume_cbm` when status is `"sized"`;
- FX output includes source, rate, checked timestamp, and currency pair when non-GBP conversion is used.

Recommended shared conventions:

```json
{
  "warnings": [
    {
      "code": "STRING_CODE",
      "message": "Human-readable explanation",
      "severity": "info|warning"
    }
  ],
  "blockers": [
    {
      "code": "STRING_CODE",
      "message": "Human-readable explanation",
      "severity": "blocked"
    }
  ],
  "evidence": [
    {
      "kind": "url|raw_text|screenshot|api_response",
      "value": "...",
      "source_url": "..."
    }
  ]
}
```

Use simple JSON. Avoid an overbuilt internal DSL.

---

## 12. Test Fixtures and Canaries

You need both unit-ish tests and live validation.

### 12.1 HMRC Rebar Canary

Build a skill/test fixture for steel rebar:

```text
Product: steel rebar / concrete reinforcement bars
Origin: CN
Expected declarable code: 7214200020
Expected basic duty: 0%
Expected VAT: 20%
Expected additional duty observation: safeguard/additional duty branch around 25% when applicable
```

Acceptance: the customs skill must not conclude "only VAT exists" if HMRC import measures contain safeguard/additional duties.

### 12.2 Current DB Canary

Run Stage 11 against the four current `candidate_pricing_assessments`.

Expected:

- At least one domestic landed-cost row for UK quotes.
- The DGU Alibaba quote either:
  - completes customs + sizing + SeaRates freight, or
  - persists a landed-cost row with clear blockers such as `SHIPMENT_DIMENSIONS_MISSING` or `SEARATES_NO_QUOTE`.
- No selected Stage 10 assessment disappears silently.

### 12.3 SeaRates Browser Canary

Use a small known shipment fixture before trying the DGU:

```text
Origin: Shanghai, China
Destination: Bristol or London, UK
Shipment: e.g. 1 CBM, 200 kg, LCL
Date: near future
```

Acceptance:

- CDP is reachable.
- SeaRates page loads.
- The skill can input route/weight/CBM/date.
- It captures a price or an explicit `SEARATES_NO_QUOTE` blocker.
- If the shown result is expired, the output sets `quote_expired = true` and still records price.

---

## 13. Validation Commands

Before editing, capture baseline:

```bash
sqlite3 state/state.db "
SELECT 'candidate_pricing_assessments=' || COUNT(*) FROM candidate_pricing_assessments;
SELECT 'landed_cost_estimates=' || COUNT(*) FROM landed_cost_estimates;
SELECT 'supplier_quotes=' || COUNT(*) FROM supplier_quotes;
SELECT 'non_gbp_quotes=' || COUNT(*) FROM supplier_quotes WHERE currency != 'GBP';
"
```

Run prior stages if needed:

```bash
cargo run -p daemon -- --pipeline procontract
```

Run Stage 11 MVP:

```bash
cargo run -p daemon -- --ingest landed-cost
```

If using queued jobs, also validate:

```bash
cargo run -p daemon -- --run-next-job
```

Post-run DB checks:

```sql
SELECT COUNT(*) AS landed_rows FROM landed_cost_estimates;

SELECT
  decision_grade,
  COUNT(*)
FROM landed_cost_estimates
GROUP BY decision_grade;

SELECT
  pc.name,
  sq.source_name,
  sq.ship_from_country,
  lce.shipping_mode,
  lce.declarable_commodity_code_10,
  lce.supplier_cost_gbp_pence,
  lce.freight_base_gbp_pence,
  lce.customs_duty_gbp_pence,
  COALESCE(lce.trade_remedy_duty_gbp_pence, 0) AS trade_remedy_duty_gbp_pence,
  lce.import_vat_gbp_pence,
  lce.economic_landed_cost_gbp_pence,
  lce.net_margin_base_bps,
  lce.decision_grade,
  lce.confidence_bps,
  lce.warnings,
  lce.blockers
FROM landed_cost_estimates lce
JOIN product_candidates pc ON pc.id = lce.product_candidate_id
JOIN supplier_quotes sq ON sq.id = lce.supplier_quote_id
ORDER BY lce.created_at, pc.name;

SELECT COUNT(*) AS orphan_quote_rows
FROM landed_cost_estimates lce
LEFT JOIN supplier_quotes sq ON sq.id = lce.supplier_quote_id
WHERE sq.id IS NULL;

SELECT COUNT(*) AS orphan_candidate_rows
FROM landed_cost_estimates lce
LEFT JOIN product_candidates pc ON pc.id = lce.product_candidate_id
WHERE pc.id IS NULL;

SELECT stage, scope_kind, COUNT(*)
FROM process_stage_heads
WHERE stage = 'landed cost'
GROUP BY stage, scope_kind;

SELECT COUNT(*) AS stage10_selected_without_landed_row
FROM candidate_pricing_assessments cpa
WHERE NOT EXISTS (
  SELECT 1
  FROM landed_cost_estimates lce
  WHERE lce.product_candidate_id = cpa.product_candidate_id
    AND lce.supplier_quote_id = cpa.selected_quote_id
);

SELECT COUNT(*) AS landed_rows_with_stale_quote_version
FROM landed_cost_estimates lce
JOIN candidate_pricing_assessments cpa
  ON cpa.product_candidate_id = lce.product_candidate_id
 AND cpa.selected_quote_id = lce.supplier_quote_id
WHERE COALESCE(
  CAST(json_extract(lce.confidence_breakdown, '$.selected_quote_row_version') AS INTEGER),
  -1
) != cpa.selected_quote_row_version;
```

Quality gates after code changes:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
```

Run targeted tests you add for:

- HMRC result parsing.
- rebar safeguard detection.
- landed-cost arithmetic.
- domestic quote row persistence.
- blocker row persistence.

---

## 14. Validation Manifest

Create:

```text
docs/pipeline/stage-11-mvp-validation-manifest.md
```

Use the style of `docs/pipeline/stage-10-validation-manifest.md`.

Include:

1. Timestamp and status.
2. Code changes summary.
3. Skill directories added.
4. How Stage 11 is triggered.
5. Baseline DB counts.
6. Post-run DB counts.
7. Per-candidate landed-cost rows.
8. Customs skill evidence summary.
9. Rebar HMRC canary result.
10. SeaRates canary result.
11. SeaRates expired-price handling result if observed.
12. FX conversion result or explanation that current DB was GBP-only.
13. Blockers/warnings distribution.
14. Stage tracking counts.
15. Exact quality-gate command results.
16. Remaining limitations.

Do not mark the manifest GREEN if Stage 11 only compiles but writes zero landed-cost rows.

---

## 15. Open Design Choices You Must Resolve

Resolve these in code and document the choices in the manifest:

1. **Trigger shape**
   - Recommended: `cargo run -p daemon -- --ingest landed-cost`.
   - Alternative: queued `workflow_jobs` plus `--run-next-job`.
2. **Domestic quote policy**
   - Do domestic UK benchmark quotes get zero freight or simple final-mile estimate?
3. **VAT policy**
   - Is unknown-recovery import VAT included in `economic_landed_cost_gbp_pence` or only in `cash_to_clear_gbp_pence`?
4. **Freight fallback policy**
   - If SeaRates fails, do you persist a blocked row with zero freight or a conservative placeholder? Prefer blocked row.
5. **Shipment sizing location**
   - Separate sizing skill vs part of SeaRates skill. Separate is cleaner because sizing is a prerequisite and reusable.
6. **FX source**
   - Google/browser skill vs deterministic simple lookup. Pick the simplest reliable path and record source/timestamp.
7. **Quantity basis**
   - Current Stage 10 has three `rfq_required` revenue rows. For supplier cost, decide how to scale unit price when tender quantity is unknown and quote quantity/MOQ exists.

---

## 16. What Not To Build

Do not build these for the MVP:

- HMRC FPO.
- Multi-source commodity classification recall.
- Product commodity-code junction/table redesign unless needed for this MVP.
- Full HS6 -> CN8 -> UK10 multi-family expansion beyond what the HMRC skill needs for one chosen code.
- Full AD/CVD/safeguard/quota interaction engine.
- TRA trade-remedy watcher.
- CBAM engine.
- Xe integration.
- Freightos integration.
- Production Stage 12 Net Margin Gate.
- L5 decision-grade output grading.
- L6 bid-workflow readiness gates.

But do preserve enough JSON detail in `duty_breakdown_json`, `warnings`, and `blockers` that L5 can improve the row later.

---

## 17. Acceptance Checklist

The task is complete only when all of these are true:

- `cargo run -p daemon -- --ingest landed-cost` or the chosen Stage 11 trigger runs without crashing.
- `landed_cost_estimates` contains real rows.
- Every attempted Stage 10 selected quote either has a landed-cost row or a documented reason in the manifest why it was excluded.
- The validation query `stage10_selected_without_landed_row` is 0, or every remaining row is explicitly justified in the manifest.
- The validation query `landed_rows_with_stale_quote_version` is 0, or stale-row reruns are explicitly documented as deferred with a follow-up issue.
- The China Alibaba DGU row is attempted and does not vanish silently.
- Customs classification is skill-backed and HMRC-sourced.
- Customs parsing catches additional import measures, demonstrated by the steel rebar canary.
- SeaRates estimation is skill-backed and CDP/browser-based.
- Shipment sizing is performed before SeaRates, or a blocker is persisted when sizing is impossible.
- Non-GBP amounts are converted through the chosen simple FX path or explicitly absent in current data.
- Stage tracking for `"landed cost"` exists.
- Desktop query still compiles against landed-cost rows.
- `cargo fmt --check` passes.
- `cargo check --all-targets` passes.
- `cargo clippy --all-targets -- -D warnings` passes.
- `docs/pipeline/stage-11-mvp-validation-manifest.md` exists and contains the validation evidence.

---

## 18. Suggested Implementation Order

1. Read all required files.
2. Add result schemas and SKILL.md files:
   - `skills/landed-cost/hmrc-customs-classification/`
   - `skills/landed-cost/shipment-sizing/`
   - `skills/landed-cost/searates-freight-estimate/`
   - optional `skills/landed-cost/simple-fx-lookup/`
3. Add Rust structs/parsers for the skill outputs.
4. Add Stage 11 candidate loader.
5. Add domestic row persistence first.
6. Add HMRC skill execution and customs result persistence.
7. Add shipment sizing skill execution.
8. Add SeaRates skill execution.
9. Add simple FX conversion for non-GBP freight.
10. Add final landed-cost assembly and `ON CONFLICT` persistence.
11. Add stage tracking and summary report.
12. Add tests/canaries.
13. Run quality gates.
14. Run the full Stage 11 MVP on current DB.
15. Write validation manifest.
16. Re-run checks until green.

Keep the first slice small: domestic rows + blocked import row is already useful. Then make the import row richer with HMRC, sizing, and SeaRates.

---

## 19. Final Response Requirements

When done, report:

- Stage 11 trigger command.
- Rows written to `landed_cost_estimates`.
- How many rows completed vs blocked.
- Customs canary result.
- SeaRates canary result.
- Validation manifest path.
- Quality gate results.
- Any remaining blockers that require operator/business input.

Do not end with "ready to push when you are." Follow repository session-completion rules.
