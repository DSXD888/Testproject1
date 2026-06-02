# Goods-Led Install/Maintenance + UK Labour-Cost Engine - Specification

> **Document Type:** Source of Truth #1 (Specification)
> **Feature:** Goods-Led Supply-and-Install / Maintenance Expansion + UK Labour-Cost Engine
> **Created:** 2026-06-02
> **Primary Source:** `docs/research/2026-06-01-goods-led-install-maintenance-pipeline-research.md`
> **Supplementary Architecture:** `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-architecture.md`

This specification is authoritative for implementation. If this document and the architecture document conflict, this specification wins.

## 1. Scope And Fixed Decisions

### 1.1 Mission

This feature ships two coupled capabilities together:

1. Widen qualification so goods-led supply-and-install and supply-and-maintain contracts surface, while service-only contracts still reject.
2. Add a modular UK installation and maintenance labour-cost engine as a first-class peer to freight, customs, FX, and VAT in landed-cost rollup.

The qualification expansion is not complete unless the labour-cost engine ships with it. The goods-led dominance rule depends on labour and permit/TM cost being visible.

### 1.2 Recorded Operator Decisions

The following decisions are fixed constraints and must not be re-decided during implementation:

| Decision | Fixed Constraint |
|---|---|
| Category breadth | "Broaden cautiously, low-reg only: festive/LED lighting, signage, fencing, playground/outdoor, blinds, shelving/racking (wood/plastic; metal split out for trade-remedy), + install on existing furniture/PPE. NO biocidal/medical/chemical/bespoke. (research §1, ADR-006)" |
| In-scope rule | "Goods value-share threshold, configurable, default 5500-6000 bps (55-60%). (research §1, ADR-001)" |
| Labour scope | "BOTH one-off install/de-install AND recurring multi-year maintenance. (research §1)" |
| Estimate grade | "Screening-grade only (±20%); never a firm quote. (research §2.3)" |

### 1.3 Operator-Confirmable Defaults

The following defaults encode the research §10 recommendations. They are operator-confirmable knobs, not open design holes:

| Knob | Default |
|---|---|
| `GOODS_DOMINANCE_FLOOR_BPS` | `6000` bps initial value, tunable against the gold set. |
| `GOODS_PRESENCE_FLOOR_BPS` | `3500` bps against `goodsDensityScore` converted to bps; low bar for "real goods present." |
| Dominance near-floor review band | `500` bps around `GOODS_DOMINANCE_FLOOR_BPS`; near-floor cost-proxy cases route to `review`, not auto-reject. |
| Missing maintenance term | Assume 1 visible year and emit `MAINTENANCE_TERM_ASSUMED`. |
| MEWP default | Operated hire, no in-house IPAF assumption. |
| Review queue visibility | Show `review` contracts in the desktop queue with labour and permit breakdown. |
| Permit responsibility default | `unknown`; include conservative permit/TM band in high estimate, base at midpoint, emit `PERMIT_RESPONSIBILITY_UNKNOWN`; operator review in the committed implementation. |
| No-list/bespoke policy | Route to `review` with exclusion reason by default; auto-reject only on high-confidence no-list CPV match. |

### 1.4 Tier Rules

Tier A is committed scope with full acceptance criteria: all MVP items plus recurring maintenance, access-equipment auto-selection, quarterly rate-refresh skill, and `overlay-labour-cost`.

Tier B is bounded in-scope research-spike scope: live per-estimate enrichment lane, installer-quote ingestion, and detailed per-authority/per-zone permit and lane-rental model. Tier B items are named and bounded in §8, but intentionally do not receive fabricated testable ACs.

### 1.5 Non-Goals

The following remain excluded:

- Service-only contracts.
- Heavily regulated goods: biocidal, medical devices, pharma, chemicals, bespoke/custom-manufactured goods.
- Final or firm pricing.
- Performing or managing installation delivery. Labour is subcontracted; the platform estimates the subcontract cost only.
- BCIS/SPON licensed feeds.

## 2. Functional Requirements

### 2.1 Qualification And Service Gates

#### FR-Q.1 Phase-1 Provisional Goods-Led Mixed Admission

**Requirement:** `determineQualificationDecision()` must preserve goods-bearing `mixed` and eligible `works` notices by routing them to `review` when goods presence is credible, without using cost-based goods value-share at this early stage.

**Acceptance Criteria:**

- `AC-FR-Q.1-a`: The Phase-1 branch is placed after the existing separable-mixed handling and before the existing `mixed|unknown -> review` branch in `packages/intelligence/src/qualification/goods-qualification.ts`.
- `AC-FR-Q.1-b`: The existing `goodsConfidenceBps < 6000 -> review` early return continues to fire before the new branch.
- `AC-FR-Q.1-c`: `mixed` or `works` with `goodsSeparability !== false` and `goodsDensityScore >= GOODS_PRESENCE_FLOOR` returns `review`, not `reject`.
- `AC-FR-Q.1-d`: `GOODS_PRESENCE_FLOOR_BPS` is a config/strategy value, default `3500`, not an inline magic literal.
- `AC-FR-Q.1-e`: `services` with no goods head noun still rejects through service-gate or qualification logic.

**Core Tests:** `TS-QUAL-006`; Appendix E Scenarios 1, 3, 9, 10, 11.

#### FR-Q.2 Separable-Mixed Pass Branch Uses Value Share

**Requirement:** The current density-only separable-mixed `pass` branch must be downgraded to defer unless the early contract-value share meets the dominance floor.

**Acceptance Criteria:**

- `AC-FR-Q.2-a`: If `addressableValuePence / totalValuePence >= GOODS_DOMINANCE_FLOOR_BPS`, separable `mixed` may pass subject to access-route reviewability.
- `AC-FR-Q.2-b`: If the early contract-value share is present and below the floor, separable `mixed` returns `review` for Phase-2/final synthesis, not `pass`.
- `AC-FR-Q.2-c`: Existing test `TS-QUAL-003` is split into `TS-QUAL-003a` with a >=6000 bps goods share expecting `pass`, and `TS-QUAL-003b` with 4000 bps expecting `review`.
- `AC-FR-Q.2-d`: `computeAddressableValue()` remains the source for the early contract-value numerator; implementation does not duplicate incompatible value-share logic.
- `AC-FR-Q.2-e`: If `computeAddressableValue()` cannot produce a contract-value numerator, the separable-mixed branch cannot pass on density alone; it returns `review` for the Phase-2 landed-cost gate.
- `AC-FR-Q.2-f`: Phase 1 never uses the landed-cost cost proxy. It may use only CPV/description goods presence, separability, confidence, access route, and the early contract-value share.

**Core Tests:** `TS-QUAL-003a`, `TS-QUAL-003b`, `TS-QUAL-008`.

#### FR-Q.3 Phase-2 Goods Dominance Gate

**Requirement:** At decision synthesis, compute goods dominance and accept goods-led mixed opportunities only when goods dominate by the configured value-share rule.

**Acceptance Criteria:**

- `AC-FR-Q.3-a`: Phase 2 runs after landed-cost labour and permit/TM fields are available, not inside early `determineQualificationDecision()`.
- `AC-FR-Q.3-b`: Preferred basis is contract-value share when `goodsAddressableValuePence` and `totalValuePence` are available.
- `AC-FR-Q.3-c`: Fallback basis is cost proxy: `round(goods_landed_cost_gbp_pence * 10000 / (goods_landed_cost_gbp_pence + installation_labour_cost_gbp_pence + permit_traffic_mgmt_cost_gbp_pence))`.
- `AC-FR-Q.3-d`: `GOODS_DOMINANCE_FLOOR_BPS` defaults to `6000` and is configurable.
- `AC-FR-Q.3-e`: Above-floor goods-led mixed contracts resolve to `pass` or `review` according to access route and confidence.
- `AC-FR-Q.3-f`: Clearly below-floor labour-dominant contracts reject as economically service-only.
- `AC-FR-Q.3-g`: Near-floor cost-proxy cases within the default 500 bps review band route to `review`, never auto-reject, because the cost proxy systematically understates goods dominance.
- `AC-FR-Q.3-h`: The final decision record logs basis type: `contract_value` or `cost_proxy`; this feature does not emit unvalidated basis labels.

**Core Tests:** `TS-COV-007`, `TS-COV-008`; Appendix E Scenarios 1, 6, 10.

#### FR-Q.4 Description-Quality Service Gate Split

**Requirement:** `gateDescriptionQuality()` must distinguish goods-led mixed descriptions from pure-service descriptions by adding a `goods_led_mixed` variant to the existing `DescriptionQualityVerdict` union.

**Acceptance Criteria:**

- `AC-FR-Q.4-a`: `DescriptionQualityVerdict` gains `{ kind: "goods_led_mixed"; installationTerms: string[]; goodsHeadNoun: string; supplySignal: string }`.
- `AC-FR-Q.4-b`: No parallel verdict taxonomy or side channel is introduced.
- `AC-FR-Q.4-c`: A goods-led mixed verdict requires both a real goods head noun and a supply/provision signal.
- `AC-FR-Q.4-d`: "Supply and installation of steel shelving" returns `goods_led_mixed`.
- `AC-FR-Q.4-e`: "Supply of maintenance kits" returns `accept`.
- `AC-FR-Q.4-f`: "Supply, install and maintain festive lighting" returns `goods_led_mixed`.
- `AC-FR-Q.4-g`: "Installation services" returns `reject_service`.
- `AC-FR-Q.4-h`: "Provision of grounds maintenance and grass cutting" returns `reject_service`.
- `AC-FR-Q.4-i`: "Managed cleaning service; contractor supplies materials" returns `reject_service`.
- `AC-FR-Q.4-j`: "Reactive and planned electrical maintenance across estate" returns `reject_service`.
- `AC-FR-Q.4-k`: "Consultancy and training" returns `reject_service`.
- `AC-FR-Q.4-l`: A new `packages/intelligence/src/customs/description-quality.test.ts` covers the full truth table with `bun:test`.

**Core Tests:** new `description-quality.test.ts` truth table; Appendix E Scenarios 5 and 7.

#### FR-Q.5 No-List And Bespoke Scope Flag

**Requirement:** The pipeline must add a lightweight no-list/bespoke runtime flag so goods-dominant but excluded categories do not silently pass the dominance gate.

**Acceptance Criteria:**

- `AC-FR-Q.5-a`: High-confidence no-list CPV families such as medical, chemical, pharma, and vehicles auto-reject or hard-review according to the no-list policy.
- `AC-FR-Q.5-b`: Text-based bespoke/custom indicators such as `bespoke`, `made-to-measure`, and `custom-manufactured` route to `review` with reason `out_of_scope_bespoke`.
- `AC-FR-Q.5-c`: Scenario 11 proves that the dominance floor alone would pass a goods-dominant bespoke window contract and that the no-list/bespoke flag catches it.
- `AC-FR-Q.5-d`: No-list hits are included in decision logging and desktop review reasons.

**Core Tests:** `TS-QUAL-007`; Appendix E Scenario 11.

#### FR-Q.6 Basis Caveat And Review Safety

**Requirement:** The cost-based goods-share proxy must be treated as a fallible eligibility signal, not ground truth.

**Acceptance Criteria:**

- `AC-FR-Q.6-a`: Specified implementation prefers contract-value share when the tender itemises goods versus labour/works.
- `AC-FR-Q.6-b`: Cost-proxy near-floor cases are never auto-rejected.
- `AC-FR-Q.6-c`: Decision manifests include the share basis, numerator, denominator, floor, review band, and final route.
- `AC-FR-Q.6-d`: Margin remains a separate economics gate using labour-inclusive landed cost.

**Core Tests:** Appendix E shared invariant; decision-basis hash tests for labour/permit field mutation.

### 2.2 Labour-Cost Engine

#### FR-L.1 Installation Requirement Extraction

**Requirement:** Product candidates that imply installation, de-installation, or maintenance must carry an `InstallationRequirement`.

**Acceptance Criteria:**

- `AC-FR-L.1-a`: `InstallationRequirement` includes `requires_installation`, `requires_maintenance`, `installation_scope`, `maintenance_schedule`, `site_access`, `at_height_m`, `uk_region`, `estimated_units`, `equipment_hint`, `permit_responsibility`, `public_realm`, and `lane_rental_zone`.
- `AC-FR-L.1-b`: Requirement data is stored on `ProductCandidate.critical_attributes_json` or additive columns without breaking existing rows.
- `AC-FR-L.1-c`: Pure goods such as Scenario 8 produce no installation requirement and labour remains 0/null.
- `AC-FR-L.1-d`: Unknown region falls back to national baseline/multiplier 1.0 and warning `LABOUR_REGION_UNKNOWN`.

**Core Tests:** Appendix E Scenarios 1, 4, 8; canary `region-unknown-01`.

#### FR-L.2 UK Installation Labour Skill

**Requirement:** Add `skills/landed-cost/uk-installation-labour-estimate` as a transient-loaded landed-cost skill.

**Acceptance Criteria:**

- `AC-FR-L.2-a`: Skill frontmatter uses `skill_id: landed-cost/uk-installation-labour-estimate`, `skill_version`, `interaction_profile: http_public`, `result_schema_version`, `model_profile`, `requires_cdp: false`, `side_effect_class: none`, and `agent_cli: claude`.
- `AC-FR-L.2-b`: The skill is not added to `CDP_BROWSER_SKILLS` or `SIGNED_IN_BROWSER_SKILLS`.
- `AC-FR-L.2-c`: The skill mirrors the freight skill result-envelope shape for `status`, low/base/high, warnings, blockers, and evidence, but does not copy freight's CDP profile.
- `AC-FR-L.2-d`: Skill output writes raw JSON to `result.json`, not Markdown.
- `AC-FR-L.2-e`: `skill_lint.rs` passes for the worker skill without requiring `checkpoint.schema.json`.
- `AC-FR-L.2-f`: `SKILL.md` includes `## Non-goals`, `## Failure Codes`, and `## Resume Strategy` sections.
- `AC-FR-L.2-g`: `## Failure Codes` enumerates `LABOUR_REGION_UNKNOWN`, `LABOUR_SCOPE_UNRESOLVED`, `ACCESS_AT_HEIGHT_ASSUMED_MEWP`, `MAINTENANCE_TERM_ASSUMED`, `RATE_TABLE_STALE`, `PERMIT_RESPONSIBILITY_UNKNOWN`, `LANE_RENTAL_ZONE`, and `TTRO_LEAD_TIME_RISK`.
- `AC-FR-L.2-h`: `SKILL.md` includes the run-manifest token block required by Appendix F.2: `manifest_version`, `skill_id`, `skill_version`, `job_id`, `session_id`, `execution_mode`, `Summary`, `Steps Taken`, `Decisions Made`, `Warnings`, and `Evidence`.
- `AC-FR-L.2-i`: The worker does not invoke, require, or document CDP browser membership; browser use is reserved for UI verification only.

**Core Tests:** `cargo test -p daemon --test skill_lint`; Appendix F.2.

#### FR-L.3 Skill I/O Contract

**Requirement:** The labour skill must accept and emit the JSON contract below.

**Acceptance Criteria:**

- `AC-FR-L.3-a`: Input includes `process_id`, `notice_id`, `product_candidate_id`, `selected_quote_id`, `destination_country`, `destination_postcode`, `installation_requirement`, `goods_landed_cost_gbp_pence`, and `rate_table_version`.
- `AC-FR-L.3-b`: Output includes `labour_status`, `uk_region`, `labour_line_items`, `access_equipment_line_items`, `labour_base_gbp_pence`, `labour_low_gbp_pence`, `labour_high_gbp_pence`, `labour_confidence_bps`, `permit_traffic_mgmt_line_items`, `permit_traffic_mgmt_base_gbp_pence`, `permit_traffic_mgmt_low_gbp_pence`, `permit_traffic_mgmt_high_gbp_pence`, `permit_responsibility`, `vat_treatment`, `excluded_cost_flags`, `evidence`, `warnings`, and `blockers`.
- `AC-FR-L.3-c`: Low/high labour defaults to base +/-20% unless a narrower evidence-backed band is available.
- `AC-FR-L.3-d`: `excluded_cost_flags` explicitly carries screening omissions such as firm-pricing exclusion, subcontractor compliance/delivery responsibility exclusion, authority-resolved permit uncertainty, VAT treatment, and any non-modelled plant/materials assumptions.
- `AC-FR-L.3-e`: `warnings` and `blockers` are arrays of objects with code, message, and severity.

**Core Tests:** labour skill `result.schema.json`; skill canary corpus.

#### FR-L.4 Skill-Local Reference Tables

**Requirement:** Labour rates and model tables must be versioned JSON files local to the skill directory.

**Acceptance Criteria:**

- `AC-FR-L.4-a`: Add `reference/uk-regional-labour-rates-2026-q2.json`.
- `AC-FR-L.4-b`: Add `reference/uk-access-equipment-bands-2026-q2.json`.
- `AC-FR-L.4-c`: Add `reference/uk-installation-profiles-2026-q2.json`.
- `AC-FR-L.4-d`: Add `reference/uk-permit-tm-bands-2026-q2.json`.
- `AC-FR-L.4-e`: Adding a new region, rate quarter, or profile row is a JSON edit plus version bump; no recompile required.
- `AC-FR-L.4-f`: Rate provenance and `rate_table_version` appear in output evidence.

**Core Tests:** canary `rate-stale-01`; `skill_lint.rs`.

#### FR-L.5 Installation Profile Resolver

**Requirement:** Resolve goods archetype to required trade(s), hours, crew, equipment, and complexity by catalogue first and constrained LLM fallback second.

**Acceptance Criteria:**

- `AC-FR-L.5-a`: Catalogue includes at least flat-pack furniture, wall-fixed shelving/racking, standard glazing/windows, festive/LED lighting, signage, fencing, playground/outdoor, CCTV/security, traffic signals as a labour-heavy boundary archetype, and made-to-measure windows as bespoke/no-list.
- `AC-FR-L.5-b`: LLM fallback may only select trades present in the rate table.
- `AC-FR-L.5-c`: Fallback results have lower confidence and evidence describing the inferred archetype.
- `AC-FR-L.5-d`: Overlay validation flags implausible mappings such as windows to electrician-only or extreme hours.
- `AC-FR-L.5-e`: Scenario 9 resolves to a different trade/equipment profile from Scenario 1.

**Core Tests:** canary `cctv-yh-install-01`; Appendix E Scenarios 1 and 9.

#### FR-L.6 One-Off Install And De-Install Modelling

**Requirement:** The labour engine must model one-off installation and separate de-installation visits.

**Acceptance Criteria:**

- `AC-FR-L.6-a`: Scenario 1 models install and seasonal de-install as distinct visits.
- `AC-FR-L.6-b`: Short-hire CPA loading is reflected by pricing separate visits independently rather than amortising across a weekly hire.
- `AC-FR-L.6-c`: Labour line items include at least `labour_type`, `trade`, `quantity_hours`, `rate_gbp_per_hour_pence`, `subtotal_gbp_pence`, `confidence_bps`, and `rate_source`.
- `AC-FR-L.6-d`: Scenario 1 NW base labour must satisfy `210000 <= installation_labour_cost_gbp_pence <= 265000`.

**Core Tests:** Appendix E Scenario 1; canary `lights-nw-2day-01`.

#### FR-L.7 Permits And Traffic-Management Resolver

**Requirement:** Basic banded permits/TM cost modelling and responsibility handling are Tier A committed scope.

**Acceptance Criteria:**

- `AC-FR-L.7-a`: Permit/TM is a distinct `PermitTrafficMgmtLineItem` component, not folded into labour.
- `AC-FR-L.7-b`: `permit_responsibility` is one of `authority_provided`, `contractor_provided`, `shared`, `unknown`.
- `AC-FR-L.7-c`: `authority_provided` prices permit/TM as 0.
- `AC-FR-L.7-d`: `unknown` includes base/high band values, may use 0 low value, and emits `PERMIT_RESPONSIBILITY_UNKNOWN`.
- `AC-FR-L.7-e`: Public-realm/high-street work produces permit/TM lines for TTRO, street works, lane rental where applicable, parking suspension, highway/scaffold licence, or physical TM bands.
- `AC-FR-L.7-f`: London/Kent lane-rental zone cases emit `LANE_RENTAL_ZONE`; TTRO cases emit `TTRO_LEAD_TIME_RISK`.

**Core Tests:** Appendix E Scenario 1 London variant; canary warning coverage.

#### FR-L.8 Recurring Maintenance And MaintenancePeriodEstimate

**Requirement:** Recurring maintenance is first-class committed scope and must not be hidden in generic metadata.

**Acceptance Criteria:**

- `AC-FR-L.8-a`: Add a `MaintenancePeriodEstimate` model/entity or equivalent additive persisted structure keyed by product candidate and estimate.
- `AC-FR-L.8-b`: The estimate records `visits_per_year`, `term_years`, visit scope, per-visit labour, annualised opex, total visible-term opex, confidence, warnings, and margin signal.
- `AC-FR-L.8-c`: Dominance is assessed on the initial supply+install capital event only.
- `AC-FR-L.8-d`: Multi-year maintenance is not capitalised into the goods dominance denominator.
- `AC-FR-L.8-e`: Missing term defaults to 1 visible year and emits `MAINTENANCE_TERM_ASSUMED`.
- `AC-FR-L.8-f`: Scenario 4 produces a separate maintenance estimate and still routes by capital-event dominance.

**Core Tests:** Appendix E Scenario 4.

#### FR-L.9 Access-Equipment Auto-Selection

**Requirement:** The skill must auto-select access equipment from height and site-access signals.

**Acceptance Criteria:**

- `AC-FR-L.9-a`: `at_height_m` and `site_access` select tower scaffold, scissor lift, boom MEWP, or truck mount according to the reference bands.
- `AC-FR-L.9-b`: Operated hire is the default when equipment is selected.
- `AC-FR-L.9-c`: Auto-selected MEWP emits `ACCESS_AT_HEIGHT_ASSUMED_MEWP`.
- `AC-FR-L.9-d`: No equipment line is produced for basic furniture assembly.

**Core Tests:** canary `lights-nw-2day-01`, `cctv-yh-install-01`, `furniture-lon-assembly-01`.

#### FR-L.10 Quarterly Rate Refresh Skill

**Requirement:** Add a quarterly refresh enrichment skill that can reseed the skill-local rate tables.

**Acceptance Criteria:**

- `AC-FR-L.10-a`: The refresh skill checks public sources and produces candidate updated JSON files for labour rates, access-equipment bands, install profiles, and permit/TM bands.
- `AC-FR-L.10-b`: It records provenance, source URLs, `as_of`, and changed cells.
- `AC-FR-L.10-c`: It behaves like a source-health/canary enrichment, not a live per-estimate scrape.
- `AC-FR-L.10-d`: Stale rate tables emit `RATE_TABLE_STALE` while still producing an estimate.
- `AC-FR-L.10-e`: Refresh does not mutate production reference files without an explicit reviewed update path.

**Core Tests:** canary `rate-stale-01`; refresh skill canary in downstream TDD.

#### FR-L.11 Labour Validation Overlay

**Requirement:** Add `skills/validation/overlay-labour-cost` as a committed validation overlay.

**Acceptance Criteria:**

- `AC-FR-L.11-a`: Overlay profile is `headless_contract`.
- `AC-FR-L.11-b`: Overlay includes `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `validator-rubric.yaml`, `canary.inputs.jsonl`, and `golden.assertions.yaml`.
- `AC-FR-L.11-c`: Overlay checks hours versus scope, rate within regional bounds, postcode/region plausibility, equipment proportionality, permit responsibility handling, and warning completeness.
- `AC-FR-L.11-d`: Overlay rejection or outage forces `review` rather than crash or silent pass.

**Core Tests:** `skill_canary_harness.rs`; overlay canary/golden assertions.

#### FR-L.12 Fail-Soft Degradation And Warning Codes

**Requirement:** Labour failures degrade confidence and route to review; they do not crash the pipeline.

**Acceptance Criteria:**

- `AC-FR-L.12-a`: `labour_status: "blocked"` produces low confidence and review routing.
- `AC-FR-L.12-b`: All warning codes are supported: `LABOUR_REGION_UNKNOWN`, `LABOUR_SCOPE_UNRESOLVED`, `ACCESS_AT_HEIGHT_ASSUMED_MEWP`, `MAINTENANCE_TERM_ASSUMED`, `RATE_TABLE_STALE`, `PERMIT_RESPONSIBILITY_UNKNOWN`, `LANE_RENTAL_ZONE`, `TTRO_LEAD_TIME_RISK`.
- `AC-FR-L.12-c`: `commercial_estimate_confidence_bps` includes labour confidence by minimum/component aggregation.
- `AC-FR-L.12-d`: Every warning code has at least one canary or E2E assertion.

**Core Tests:** Appendix F.1 warning-code coverage.

### 2.3 Rollup, Schema, Sync, And Decision Synthesis

#### FR-R.1 LandedCostEstimate Labour And Permit Fields

**Requirement:** `LandedCostEstimate` must expose labour and permit/TM as explicit nullable/default-safe fields.

**Acceptance Criteria:**

- `AC-FR-R.1-a`: Add `installation_labour_cost_gbp_pence`, `installation_labour_low_gbp_pence`, `installation_labour_high_gbp_pence`, and `installation_labour_confidence_bps`.
- `AC-FR-R.1-b`: Add `permit_traffic_mgmt_cost_gbp_pence`, `permit_traffic_mgmt_low_gbp_pence`, `permit_traffic_mgmt_high_gbp_pence`, and `permit_responsibility`.
- `AC-FR-R.1-c`: Add or persist `goods_landed_cost_gbp_pence` so the ratio is reconstructable from a single row.
- `AC-FR-R.1-d`: Fields are nullable or default 0 so legacy explicit INSERT tests continue to run.
- `AC-FR-R.1-e`: `permit_traffic_mgmt` remains separate from labour in schema, rollup, manifest, and UI.

**Core Tests:** daemon persistence round-trip; legacy `customs_fx_freight_engine.rs` and `decision_basis_hash.rs`.

#### FR-R.2 Labour-Inclusive Rollup At Both Sites

**Requirement:** Labour and permit/TM base costs must be summed into landed cost for both domestic and import estimates.

**Acceptance Criteria:**

- `AC-FR-R.2-a`: `build_domestic_landed_cost_estimate` adds `installation_labour_cost_gbp_pence` and `permit_traffic_mgmt_cost_gbp_pence` to the supplier-cost base when an `InstallationRequirement` exists.
- `AC-FR-R.2-b`: `build_import_landed_cost_estimate` adds those same base fields after supplier/freight/duty/import VAT.
- `AC-FR-R.2-c`: Low/high net margin bands use labour/permit low/base/high consistently.
- `AC-FR-R.2-d`: Scenario 8 has labour/permit 0/null and economic landed cost unchanged.
- `AC-FR-R.2-e`: Saturating integer-pence arithmetic is used throughout.

**Core Tests:** Rust daemon unit tests for both functions; Appendix E Scenarios 1 and 8.

#### FR-R.3 Live Loop Labour Invocation

**Requirement:** `run_landed_cost_live_loop` must transient-load and invoke the labour skill in the landed-cost sequence.

**Acceptance Criteria:**

- `AC-FR-R.3-a`: Add `load_skill(repo_root.join("skills/landed-cost/uk-installation-labour-estimate"), ...)`.
- `AC-FR-R.3-b`: Import flow invokes job kind `landed_cost_uk_installation_labour` after FX conversion and before `build_import_landed_cost_estimate`.
- `AC-FR-R.3-c`: Domestic flow invokes labour after supplier-cost basis is computed and before `build_domestic_landed_cost_estimate` when an installation requirement exists.
- `AC-FR-R.3-d`: No `registry.rs` `SourceDefinition` edit is required or allowed for this skill.
- `AC-FR-R.3-e`: Skill versions, session IDs, statuses, and run counters include `uk_installation_labour`.
- `AC-FR-R.3-f`: The labour skill is skipped for candidates with no `InstallationRequirement` and records zero labour.

**Core Tests:** live-loop smoke target; `skill_lint.rs`; Appendix E deterministic and canary tiers.

#### FR-R.4 PipelineAssessment Goods Share And Coverage

**Requirement:** Pipeline assessment must carry goods-share and labour/permit coverage metrics.

**Acceptance Criteria:**

- `AC-FR-R.4-a`: Add `goods_value_share_bps`.
- `AC-FR-R.4-b`: Add `installation_labour_coverage_bps` and `installation_labour_value_coverage_bps`.
- `AC-FR-R.4-c`: Add permit/TM coverage or include permit coverage explicitly in labour coverage naming if the schema chooses a combined metric; UI label must remain separate.
- `AC-FR-R.4-d`: Missing labour estimates are conservative and route to review.
- `AC-FR-R.4-e`: Coverage math mirrors existing basis-point conventions in `coverage-rollup.ts`.

**Core Tests:** `TS-COV-007`, `TS-COV-008`, `TS-COV-009`.

#### FR-R.5 Contracts Schema Extensions

**Requirement:** `packages/contracts-schema/src/index.ts` must describe the new row fields and installation requirement shape.

**Acceptance Criteria:**

- `AC-FR-R.5-a`: `ProductCandidateRow` exposes `InstallationRequirement` via `critical_attributes_json` typing or additive columns.
- `AC-FR-R.5-b`: `LandedCostEstimateRow` exposes labour, permit/TM, and goods-only subtotal fields.
- `AC-FR-R.5-c`: `PipelineAssessmentRow` exposes goods share and labour/permit coverage fields.
- `AC-FR-R.5-d`: `SupplierQuoteRow.quote_class` remains the extension point for Tier B installer quotes.
- `AC-FR-R.5-e`: Add `MaintenancePeriodEstimateRow` or an equivalent first-class contracts-schema type for the recurring-maintenance entity; do not hide maintenance-period fields inside generic `Json`.

**Core Tests:** package type/build tests.

#### FR-R.6 SQLite, Supabase, And Schema Epoch

**Requirement:** Schema changes must be additive, projection-aware, and epoch-consistent.

**Acceptance Criteria:**

- `AC-FR-R.6-a`: SQLite migration is `migrations/0014_installation_labour_fields.sql`.
- `AC-FR-R.6-b`: Migration adds columns with nullable/default-0 behavior for legacy direct INSERT tests.
- `AC-FR-R.6-c`: Supabase projection migration is added under `infra/supabase/migrations/0010_*.sql` for `landed_cost_estimates_viewer`.
- `AC-FR-R.6-d`: `pipeline_assessments` is already a Supabase projection table and a `READABLE_PROJECTION_TABLES` entry, so the same Supabase migration must project `goods_value_share_bps`, `goods_value_share_basis`, and labour/permit coverage fields.
- `AC-FR-R.6-e`: Schema epoch bumps from 9 to 10 in `crates/daemon/src/db/sqlite.rs`.
- `AC-FR-R.6-f`: `apps/desktop/src-tauri/src/main.rs` `EXPECTED_SCHEMA_EPOCH` bumps from 9 to 10.
- `AC-FR-R.6-g`: `apps/desktop/src-tauri/src/daemon_bridge.rs` reconciles from 8 to 10.
- `AC-FR-R.6-h`: `apps/desktop/src-tauri/src/query_service.rs` reconciles from 8 to 10.
- `AC-FR-R.6-i`: Supabase `schema_metadata` bumps from 9 to 10.
- `AC-FR-R.6-j`: `customs_fx_freight_engine.rs` schema assertion passes at epoch 10.

**Core Tests:** full AGENTS stage gate; SQLite schema integration tests.

#### FR-R.7 Decision Basis Hash Participation

**Requirement:** Labour and permit/TM inputs must participate in decision-basis hashing so bid readiness can detect changed cost basis.

**Acceptance Criteria:**

- `AC-FR-R.7-a`: Decision-basis vector includes goods-only landed subtotal, labour base/low/high/confidence, permit/TM base/low/high/responsibility, goods-share basis, and goods-share bps.
- `AC-FR-R.7-b`: Mutating any labour or permit field changes the hash.
- `AC-FR-R.7-c`: Historical rows with null/default labour fields hash deterministically.

**Core Tests:** extend `decision_basis_hash.rs`.

### 2.4 UI And Operator Surfacing

#### FR-U.1 Manifest Viewer Cost Breakdown

**Requirement:** Run manifests and manifest viewer must show labour and permit/TM lines.

**Acceptance Criteria:**

- `AC-FR-U.1-a`: Cost table includes "Installation labour" with low/base/high/confidence.
- `AC-FR-U.1-b`: Cost table includes "Permits & traffic management" with low/base/high and `permit_responsibility`.
- `AC-FR-U.1-c`: Manifest includes region, installation scope, equipment assumptions, warning codes, goods-share basis, floor, and decision route.
- `AC-FR-U.1-d`: Screening-grade, VAT treatment, and "what is excluded" flags are visible in the manifest, including that the estimate is not a firm quote and does not price Sorellon's direct delivery of installation work.

**Core Tests:** Appendix F.3 agent-browser UI flow.

#### FR-U.2 Contract Card Review Metrics

**Requirement:** Contract cards must surface labour coverage and review context.

**Acceptance Criteria:**

- `AC-FR-U.2-a`: Add `labourCoverageBps` metric row to `packages/ui/contract-card-frame.tsx`.
- `AC-FR-U.2-b`: `review` goods-led contracts remain visible in the desktop queue.
- `AC-FR-U.2-c`: Review cards display whether the review is due to near-floor cost proxy, unresolved permit responsibility, no-list/bespoke flag, or labour uncertainty.
- `AC-FR-U.2-d`: Web read-only surfaces remain non-mutating.

**Core Tests:** Appendix F.3 agent-browser UI flow; web read-only E2E if projection changes.

### 2.5 Validation, Gold Set, And Quality Gates

#### FR-V.1 Gold-Set Scenario Corpus

**Requirement:** Appendix A's 11 scenarios are the acceptance corpus.

**Acceptance Criteria:**

- `AC-FR-V.1-a`: Scenario 1 festive lights supply/install/de-install passes or reviews with labour, MEWP, permit unknown, and NW hard labour assert.
- `AC-FR-V.1-b`: Scenario 2 office/classroom furniture supply/install passes or reviews with fitter/handyperson and no MEWP.
- `AC-FR-V.1-c`: Scenario 3 wooden shelving/racking supply/fit reviews conservatively; metal twin preserves customs AD/CVD handling.
- `AC-FR-V.1-d`: Scenario 4 playground supply/install/maintenance produces separate maintenance estimate and reviews on capital-event dominance.
- `AC-FR-V.1-e`: Scenario 5 grounds maintenance services rejects.
- `AC-FR-V.1-f`: Scenario 6 managed cleaning service rejects through low goods-share despite minor consumables.
- `AC-FR-V.1-g`: Scenario 7 electrical maintenance/repair rejects.
- `AC-FR-V.1-h`: Scenario 8 pure A4 copier paper passes unchanged with labour 0.
- `AC-FR-V.1-i`: Scenario 9 CCTV supply/install passes or reviews with distinct resolver output.
- `AC-FR-V.1-j`: Scenario 10 traffic signals reject via Phase-2 dominance floor, not category gate.
- `AC-FR-V.1-k`: Scenario 11 bespoke windows route to review via no-list/bespoke flag.

**Core Tests:** `crates/daemon/tests/installation_labour_engine.rs`.

#### FR-V.2 E2E Manifests And Scenario 1 Hard Assert

**Requirement:** Appendix E manifests become runnable deterministic E2E tests.

**Acceptance Criteria:**

- `AC-FR-V.2-a`: Add `crates/daemon/tests/installation_labour_engine.rs`.
- `AC-FR-V.2-b`: Tests seed real SQLite rows using existing support helpers, not mocks.
- `AC-FR-V.2-c`: Every scenario asserts `goods_value_share_bps` is reconstructable from persisted row fields using `round(goods_landed_cost_gbp_pence * 10000 / (goods_landed_cost_gbp_pence + installation_labour_cost_gbp_pence + permit_traffic_mgmt_cost_gbp_pence))`.
- `AC-FR-V.2-d`: Scenario 1 asserts `210000 <= installation_labour_cost_gbp_pence <= 265000`.
- `AC-FR-V.2-e`: Every scenario asserts integer pence and bps conventions.

**Core Tests:** `cargo test -p daemon --test installation_labour_engine`.

#### FR-V.3 Labour Skill Canary Corpus

**Requirement:** Appendix F canary corpus must be implemented and pinned to warning coverage.

**Acceptance Criteria:**

- `AC-FR-V.3-a`: Fixture directory is `tests/fixtures/calibration/landed-cost/uk-installation-labour/`.
- `AC-FR-V.3-b`: File-set includes `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`, `metadata.json`, and `manifest.md`.
- `AC-FR-V.3-c`: `metadata.json.row_count` equals JSONL line count.
- `AC-FR-V.3-d`: `manifest.md` mentions each `Scenario`.
- `AC-FR-V.3-e`: Canary cases include `lights-nw-2day-01`, `lights-lon-2day-01`, `cctv-yh-install-01`, `furniture-lon-assembly-01`, `region-unknown-01`, `scope-unresolved-01`, and `rate-stale-01`.
- `AC-FR-V.3-f`: Every §5.6 warning code is asserted by at least one canary or E2E case.

**Core Tests:** `cargo test -p daemon --test skill_canary_harness`.

#### FR-V.4 L0 Whitelist Expansion Gate

**Requirement:** The L0 whitelist doc must be updated and every new category must pass its own gold-set gate before production.

**Acceptance Criteria:**

- `AC-FR-V.4-a`: `docs/strategy/layers/layer-0-gold-set-evaluation.md §1.6` adds the low-reg categories from the recorded decision.
- `AC-FR-V.4-b`: Metal shelving/racking is split from wood/plastic and marked trade-remedy-heavy.
- `AC-FR-V.4-c`: Biocidal, medical, chemical, bespoke, and other no-list items remain excluded.
- `AC-FR-V.4-d`: Each new category has a labelled gold-set slice before entering production.
- `AC-FR-V.4-e`: Promotion requires 100% negative and regression controls plus acceptance precision at the agreed floor.

**Core Tests:** gold-set corpus gate in `installation_labour_engine.rs`.

#### FR-V.5 Full Quality Gate

**Requirement:** A stage touching this feature is not green until the full mixed Rust/TypeScript gate runs.

**Acceptance Criteria:**

- `AC-FR-V.5-a`: `cargo fmt --check` passes.
- `AC-FR-V.5-b`: `cargo check --all-targets` passes.
- `AC-FR-V.5-c`: `cargo clippy --all-targets -- -D warnings` passes.
- `AC-FR-V.5-d`: `cargo test -p daemon --bin daemon` passes.
- `AC-FR-V.5-e`: Every `crates/daemon/tests/*.rs` integration target runs.
- `AC-FR-V.5-f`: `bun run test` or equivalent package test command passes for TypeScript changes.
- `AC-FR-V.5-g`: UI/browser verification uses `/agent-browser` for rendered manifest/contract-card checks.

## 3. User Stories

| ID | Story | Mapped Requirements |
|---|---|---|
| US-1 | As the operator, I want goods-led supply-and-install contracts to surface so I can build a viable bid cadence. | FR-Q.1, FR-Q.2, FR-Q.3, FR-Q.4 |
| US-2 | As the operator, I want labour, access-equipment, and permit/TM costs in landed cost so I can screen commercial viability. | FR-L.2-FR-L.12, FR-R.1-FR-R.3 |
| US-3 | As the operator, I want service-only contracts to still reject so I do not waste bid effort. | FR-Q.3, FR-Q.4, FR-Q.5, FR-V.1 |
| US-4 | As the operator, I want to tune goods dominance without code changes. | FR-Q.1, FR-Q.3 |
| US-5 | As a maintainer, I want new installable categories to be onboarded through reference data and gold sets, not daemon rewrites. | FR-L.4, FR-L.5, FR-V.4 |

## 4. E2E Test Scenarios

All E2E tests use real SQLite state, existing daemon support helpers, and the real rollup/gate functions. No HTTP mocks or stubbed rendered components satisfy these scenarios.

| Scenario | Expected Outcome |
|---|---|
| 1 Festive lights supply/install/de-install, NW | PASS/REVIEW; labour hard assert `210000-265000`; permit unknown high band. |
| 2 Furniture supply/install, London | PASS/REVIEW; fitter/handyperson; no MEWP; London multiplier applied. |
| 3 Wooden shelving/racking supply/fit | REVIEW; joiner/fitter; metal twin preserves customs remedy. |
| 4 Playground supply/install/maintenance | REVIEW if capital event goods-dominant; separate `MaintenancePeriodEstimate`. |
| 5 Grounds maintenance services | REJECT at service gate. |
| 6 Managed cleaning service | REJECT via low goods share. |
| 7 Electrical maintenance/repair | REJECT at service gate. |
| 8 Pure A4 copier paper | PASS unchanged; labour 0/null; cost unchanged. |
| 9 CCTV supply/install | PASS/REVIEW; security/electrical resolver; YH multiplier. |
| 10 Traffic signals | REJECT via Phase-2 dominance floor. |
| 11 Bespoke windows | REVIEW via no-list/bespoke flag. |

## 5. Dependencies

- Existing TypeScript qualification, description-quality, and coverage-rollup packages.
- Existing daemon landed-cost live loop and `run_landed_cost_skill` machinery.
- Existing SQLite schema migration and Supabase projection conventions.
- Existing skill lint and canary harnesses.
- Existing desktop/web UI shared components.
- `/agent-browser` skill for UI verification.

## 6. Risks And Mitigations

| Risk | Severity | Mitigation |
|---|---|---|
| Service-only contracts leak through fuzzy text gate | High | `goods_led_mixed` requires goods head noun and supply signal; eight-row truth table; negative controls must be 100% green. |
| Cost proxy false-rejects genuinely goods-led contracts | High | Prefer contract-value basis; near-floor cost-proxy cases route to review; log basis and denominator. |
| Labour estimate too rough for final pricing | Medium | Screening-grade labels, +/-20% band, confidence bps, overlay validation, not presented as firm quote. |
| Permits/TM dominate spread | Medium | Separate permit fields and manifest rows; responsibility flag; conservative `unknown` default. |
| Schema epoch mismatch breaks desktop reads | High | Four-constant epoch checklist plus Supabase metadata/projection migration. |
| Existing direct INSERT tests break | Medium | Additive nullable/default-0 columns only. |

## 7. Success-Criteria Coverage Matrix

| Research §2.3 Criterion | Requirement IDs |
|---|---|
| Shelving supply/install classified goods-led mixed and reaches pass/review if share >= floor | FR-Q.1, FR-Q.3, FR-Q.4, FR-V.1 |
| Grounds maintenance services still rejected | FR-Q.4, FR-V.1 |
| Pure A4 copier paper still passes unchanged | FR-L.1, FR-R.2, FR-V.1 |
| In-scope install includes non-zero labour 3-point band and confidence | FR-L.3, FR-L.6, FR-R.1, FR-V.2 |
| Goods value-share threshold read from config | FR-Q.1, FR-Q.3 |
| Region derived from delivery location changes labour estimate | FR-L.1, FR-L.4, FR-L.9, FR-V.3 |
| Recurring maintenance produces multi-period estimate | FR-L.8, FR-V.1 |
| Screening-grade +/-20%, ex-VAT, not firm quote | FR-L.3, FR-U.1 |
| Labour skill failures fail-soft and route review | FR-L.11, FR-L.12 |
| New region/rate quarter is JSON edit plus version bump | FR-L.4, FR-L.10 |

## 8. Tier B Bounded Future Scope

These items are in scope for tracking and strategy, but require research spikes before full implementation ACs. Do not silently drop them, and do not invent test assertions before the spike resolves the open questions.

### FR-Z.1 Live Per-Estimate Enrichment Lane

Scope statement: operator-triggered high-value-bid enrichment that can rerun or augment `run_landed_cost_live_loop` for one estimate using fresher labour/equipment/permit evidence.

Extension point: operator-triggered re-run of `run_landed_cost_live_loop` with enriched `job_parameters` and a distinct basis marker.

Research reference: research §10 "Requiring further research" live/manual lane and at-height/attachment extraction questions.

### FR-Z.2 Installer-Quote Ingestion

Scope statement: ingest a UK installer subcontract quote and use it as evidence that can override or calibrate table-based labour estimates.

Extension point: `SupplierQuoteRow.quote_class = "installer_quote"` and selected-quote/value-share synthesis.

Research reference: research §10 "Sub-contractor quote ingestion as quote_class = installer_quote."

### FR-Z.3 Detailed Per-Authority/Per-Zone Permit And Lane-Rental Model

Scope statement: refine Component A3 from banded permit/TM ranges into detailed authority/zone-specific fee modelling.

Extension point: `uk-permit-tm-bands-*` reference data and `PermitTrafficMgmtLineItem` resolver.

Research reference: research §10 "Traffic-management / pavement-licence cost sub-model for highway/high-street jobs." The Tier A implementation still ships basic banded permits/TM plus `permit_responsibility`.
