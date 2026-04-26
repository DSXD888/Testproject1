# Stage 11 Landed Cost — Deep Research Handoff

ULTRATHINK and be meticulous. This is a **research-only** handoff. You will NOT write production code or modify the database. You WILL produce a comprehensive research document that becomes the source of truth for any future Stage 11 build work.

## FRESH-EYES CURRENT-STATE OVERRIDE (2026-04-26)

This handoff has now been executed once. The resulting research document exists at `docs/research/stage-11-landed-cost-research.md` and should be treated as the first source to read before re-running or extending Stage 11 research.

Current verified Stage 11 state:

```text
candidate_pricing_assessments=4
landed_cost_estimates=0
product_commodity_codes table exists? 0
No production HMRC/Freightos/SeaRates/Xe/FPO integration found
run_deterministic_pipeline() currently stops at gross margin + relate; no Stage 11 function is wired
```

If this handoff is reused, the mission is **refresh and verify the existing research document**, not overwrite it from scratch. Re-check live code/DB/external docs, preserve sourced claims, and explicitly call out any drift from the 2026-04-26 research doc.

## MISSION

Produce, refresh, or verify the single definitive research document covering everything about **Stage 11: Landed Cost** in this platform. If `docs/research/stage-11-landed-cost-research.md` already exists, treat it as the base document: re-check every important claim against current docs/code/DB/external sources, update drift, and preserve source citations. The document must answer every question a future build agent could ask, sourced from the actual documentation and codebase — not from training-data assumptions.

The user wants to understand:
1. **What Stage 11 is supposed to do** — the full conceptual scope, per the layer documentation
2. **How costs are estimated** — supplier cost + freight + duty + VAT + final mile + other fees, and how each is computed
3. **How duties are calculated** — HMRC APIs, commodity classification, multi-source recall, measure interaction, trade remedies, CBAM
4. **How logistics/freight is estimated** — flat-rate (screening-grade L1-L3) vs API-driven (decision-grade L5)
5. **How delivery time is estimated** — lead times, shipping mode, transit time
6. **What tools/APIs are used** — HMRC Trade Tariff, HMRC FPO, HMRC FX, Xe spot, Freightos, SeaRates, etc.
7. **What's currently built** (we're at L1-L3, fixture-only for Stage 11)
8. **What's NOT yet built** — the gap to L1-L3 minimum-viable Stage 11, and the further gap to L5 decision-grade

The output is a markdown research document at `docs/research/stage-11-landed-cost-research.md`. It will be reviewed by the user and used to plan the actual build handoff.

```
┌─────────────────────────────────────────────────────────────────┐
│                    THE RESEARCH PROCESS                          │
│                                                                  │
│   1. Read ALL relevant documentation (strategy + research docs) │
│   2. Map ALL relevant codebase surfaces (schema, fixtures,       │
│      any existing HMRC/freight code)                             │
│   3. Query the live DB for current state                         │
│   4. Synthesize into a single research document                  │
│   5. Self-verify: every claim cited to a source                  │
│                                                                  │
│   NO CODE CHANGES. NO DB MUTATIONS. RESEARCH ONLY.               │
└─────────────────────────────────────────────────────────────────┘
```

---

## CONTEXT: Where We Are in the Project

```
Layers implemented: L1, L2, L3
Layers NOT implemented: L4, L5, L6

Stage 11 status (current):
  - Schema exists (landed_cost_estimates table from L0)
  - Fixture-materialization code exists in main.rs (test path only)
  - 0 rows in landed_cost_estimates (no real Stage 11 has ever run)
  - No HMRC API integration
  - No freight API integration
  - No FX rate fetching
  - No commodity classification

Per the layer docs (L1 §1.10):
  - L1 ships "screening-grade" landed cost
  - L5 upgrades to "decision-grade"
  - Decision-grade fields exist as nullable columns from L0

This research must distinguish:
  • L1-L3 minimum-viable Stage 11 scope (screening-grade)
  • L5 decision-grade upgrades
  • Constitutional conservative defaults shared across both
```

---

## RESEARCH STRATEGY (read this before Phase 1)

### Documentation files are LARGE — grep first, read sections second.

Approximate file sizes:
- `docs/strategy/layers/layer-0-gold-set-evaluation.md` — ~4170 lines
- `docs/strategy/layers/layer-1-thin-local-runner.md` — ~1590 lines
- `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — large
- `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` — ~8000+ lines
- `docs/strategy/layer-strategy.md` — ~1400 lines

**Do NOT read these top-to-bottom.** Workflow:

```bash
# Step 1: find all Stage 11 / landed-cost mentions across docs
grep -rn "Stage 11\|landed.cost\|landed_cost\|customs.duty\|customs duty\|tariff" docs/strategy/ docs/research/ 2>/dev/null

# Step 2: for each hit, use Read with offset+limit to grab the surrounding ~50 lines
# Step 3: only read full files when grep shows dense Stage-11 content (e.g., L5 doc)
```

This keeps context manageable while ensuring no Stage 11 mention is missed.

### Skip directives — what is NOT in scope

These are real concerns in the strategy docs but **not Stage 11 concerns**. Skip them unless they directly cite landed cost:
- Worker-validator system / validation_verdicts
- Agent runtime / PTY / skill execution
- Sync engine / outbox / projections
- Operator decision UI / Tauri desktop
- Bid workflow automation (L6)
- Anything about ingestion sources (L1-L3 procurement-ingestion skills)

If a section is about one of the above and only mentions landed cost incidentally, note the cross-reference but don't deep-read it.

### Citation format — every claim must be sourced

When writing the research document, cite sources inline:
- Doc citation: `[L5 §3.2]` (layer 5 doc, section 3.2)
- Doc with line range: `[L0 §4.6, lines 2400-2440]`
- Codebase: `[main.rs:9080]` or `[main.rs:9080-9100]`
- Migration: `[migrations/0001_sqlite_state_initial.sql]`
- External: full URL

If you can't cite a source, state "**[Unsourced — inferred]**" and add it to Open Questions.

### Source-of-truth hierarchy

1. **For "what's actually built":** The codebase wins. Always.
2. **For "what's intended":** The strategy docs win. The research doc is older and may have been superseded.
3. **For schema constraints:** The migration files win, even if L0 doc says otherwise. The migration is the executable truth.
4. **When docs disagree with each other:** Document the disagreement in Open Questions, don't pick a side silently.

---

## PHASE 1: Documentation Research

Read these documents using the grep-first strategy above. Take notes on every Stage 11 reference.

### 1A. The strategy layer docs (`docs/strategy/`)

**Effort allocation hint:** Stage 11 content is heavily weighted to L0 (schema) and L5 (decision-grade upgrades). L2/L3/L4 likely have minimal direct Stage 11 changes. Spend ~50% of doc reading time on L0 + L5, ~30% on L1, ~20% on the rest.

| File | Priority | What to look for |
|------|----------|-----------------|
| `layers/layer-0-gold-set-evaluation.md` | **HIGH** | The original `landed_cost_estimates` schema definition, all nullable columns, the constitutional defaults |
| `layers/layer-5-expanded-commercial-intelligence.md` | **HIGH (THE BIG ONE)** | Decision-grade upgrade — multi-source recall, dual FX, freight APIs, exporter resolution, measure interaction, CBAM, trade remedy monitoring |
| `layers/layer-1-thin-local-runner.md` | **MEDIUM** | What "screening-grade" means in L1, what's stubbed (§1.10 stub list) |
| `layer-strategy.md` | **MEDIUM** | Stage Scope Matrix entry for Stage 11 (scope grain, commit semantics, schema activation per layer) |
| `layers/layer-6-request-pipeline-automation.md` | LOW | L5→L6 readiness gates — landed cost blockers that prevent bid workflow |
| `layers/layer-2-single-operator-desktop.md` | LOW | Any landed cost UI surfaces (cost breakdown display, scenario bands) |
| `layers/layer-3-operational-hardening.md` | LOW | Any L3 changes to landed cost (probably none) |
| `layers/layer-4-shared-visibility.md` | LOW | Sync semantics for landed cost rows |
| `cross-reference-index.md` | LOW | Cross-cutting Stage 11 references |
| `README.md` | LOW | Top-level orientation only — skip if time-constrained |

### 1B. The original research doc

`docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`

This is the foundational research. Find:
- §4.5 (or similar section) — customs classification, FPO, multi-source recall
- §4.6 (or similar) — FX rates, dual FX engine
- §5.7 — pipeline integration and landed cost stage details
- ADRs — any ADR specifically about landed cost, customs, or freight
- Appendices — tables of duty types, CBAM scope, exporter rate basis

### 1C. Harmonymaxer artifacts

`docs/harmonymaxer/` and `docs/planmaxer/` — find any iteration that touched landed cost (look for "customs", "landed", "freight", "HMRC", "FPO", "tariff" in filenames). The customs-classification expert review (planmaxer iteration 003) is particularly relevant.

### 1D. Specific topics to extract from the docs

For each topic, find the authoritative definition AND the current implementation status. **Important:** when grepping the codebase or migrations, use **snake_case** (Rust/SQL convention). When grepping the docs, try BOTH camelCase and snake_case variants since docs sometimes use TypeScript-style names.

| # | Topic | Suggested grep terms |
|---|-------|---------------------|
| 1 | Customs valuation methods | `customs_valuation_method`, `transaction value`, `proxy method`, `fallback method` |
| 2 | Valuation mode (estimation vs declaration) | `valuation_mode`, `1_proxy`, `1_actual` |
| 3 | Conservative defaults | `conservative default`, `MFN`, `residual`, `out-of-quota`, `standard VAT` |
| 4 | Exporter rate basis | `exporter_rate_basis`, `named_exporter`, `annex_exporter`, `residual_default` |
| 5 | Multi-source classification recall (5 parallel sources) | `multi-source recall`, `FPO`, `CPV.*chapter`, `tariff keyword`, `ATR`, `codebook` |
| 6 | Multi-family expansion (HS6 → CN8 → UK 10-digit) | `HS6`, `CN8`, `UK 10`, `UK10`, `multi-family`, `greedy descent` |
| 7 | Measure interaction (AD/CVD/safeguard/quota) | `measure interaction`, `anti-dumping`, `AD/CVD`, `safeguard`, `quota stack` |
| 8 | Duty expression preservation (compound, specific, ad valorem) | `compound duty`, `specific duty`, `ad valorem`, `dutyPct`, `duty_breakdown` |
| 9 | Supplementary units (kg, m², pieces) | `supplementary_unit`, `additional_code` |
| 10 | VAT rate determination (separate from recovery) | `vat_rate_state`, `vat_recovery`, `import_vat_treatment` |
| 11 | Trade remedy monitoring | `trade_remedy_watch_state`, `trade_remedy_state`, `TRA`, daily watcher |
| 12 | Classification freshness | `classification_fresh_until`, `tariff_checked_at`, `classificationFreshUntil`, `tariffCheckedAt` |
| 13 | CBAM scope (carbon border adjustment) | `cbam`, `cbam_exposure_flag`, `cbam_estimate` |
| 14 | FX rate model (HMRC monthly + Xe spot) | `customs_fx_rate`, `commercial_fx_rate`, `fx_basis_delta_bps`, `dual FX` |
| 15 | Freight engine (flat L1-L3, APIs L5) | `freight_low`, `freight_base`, `freight_high`, `Freightos`, `SeaRates`, `shipping_mode` |
| 16 | Customs branch divergence (hard blocker threshold) | `customs_cost_spread_bps`, `branch divergence`, `customs branch` |
| 17 | Decision-grade output values | `decision_grade`, `'review'`, `'rfq_required'`, `'broker_required'` |
| 18 | Hard blockers (rejection vs review) | `blockers`, `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING` |
| 19 | Lead time / transit time | `lead_time_days`, `transit_time`, `delivery time` |
| 20 | Working capital / cash flow | `cash_outlay_gbp_pence`, `cash_to_clear_gbp_pence`, `working_capital_cost` |

---

## PHASE 2: Codebase Research

### 2A. Schema definition

```bash
sqlite3 state/state.db ".schema landed_cost_estimates"
```

Document EVERY column (~50+ columns). For each:
- Name and type
- L0 nullable / L1 populated / L5 populated
- What it represents
- Where its value comes from (computed / fetched / configured / defaulted)

Cross-reference against `migrations/0001_sqlite_state_initial.sql` to find the exact CREATE TABLE statement and any L0 documentation in the migration comments.

### 2B. Existing fixture pattern

Find the fixture-materialization code that demonstrates the intended Stage 11 shape:

```bash
grep -n "INSERT INTO landed_cost_estimates" crates/daemon/src/main.rs
```

Document the fixture's hardcoded values — these are NOT real Stage 11 outputs but they show the column population pattern.

### 2C. Search for any existing HMRC / freight / FX code

Run these targeted greps (broad-then-narrow):

```bash
# Pass 1: integration-y patterns (likely zero hits, but verify)
# Note: \bFPO\b uses word boundaries to avoid false positives in random tokens
grep -rnE "trade-tariff|HMRC|Freightos|SeaRates|api\.xe\.com|\\bFPO\\b" \
  crates/daemon/src/ packages/ 2>/dev/null

# Pass 2: schema/struct references for cost components (snake_case)
grep -rn "landed_cost_estimates\|customs_duty\|trade_remedy\|cbam\|exporter_rate_basis\|valuation_mode\|supplementary_unit" \
  crates/daemon/src/ packages/ 2>/dev/null

# Pass 3: any commodity-code handling
grep -rnE "commodity_code|\\bhs6\\b|\\bcn8\\b|cpv.*chapter|tariff_keyword" \
  crates/daemon/src/ packages/ 2>/dev/null
```

For each hit, document:
- File + line
- Whether it's a struct field, query, fixture-write, downstream-read, or API client
- Production vs test code (test code lives under `#[cfg(test)]` or `tests/` directories)

Expected findings (verify, don't assume):
- `landed_cost_estimates` references in the fixture path (test code)
- TypeScript intelligence functions in `packages/intelligence/` for margin/coverage that READ landed cost data downstream
- **Probably zero** real HMRC/Freightos/Xe API client code
- **Possibly some** struct definitions in TypeScript even though no Rust integration exists yet

If you find significant unexpected integration code, that changes the build picture — flag it prominently.

### 2D. The TypeScript intelligence side

`packages/intelligence/src/margins/` — does any TypeScript code consume `landed_cost_estimates` for downstream computation? Map the consumers so we understand the contract Stage 11 must produce.

### 2E. Database state

```bash
sqlite3 state/state.db "
SELECT 'landed_cost_estimates rows: ' || COUNT(*) FROM landed_cost_estimates;
SELECT 'product_commodity_codes table exists? ' || COUNT(*) FROM sqlite_master WHERE name = 'product_commodity_codes';
SELECT 'Tables referencing landed cost concepts:';
SELECT name FROM sqlite_master WHERE type = 'table' AND (name LIKE '%commodity%' OR name LIKE '%customs%' OR name LIKE '%landed%' OR name LIKE '%duty%' OR name LIKE '%tariff%' OR name LIKE '%fx%');
"
```

### 2F. Related but separate stages

Stage 11 sits between Stage 10 (Gross Margin Gate) and Stage 12 (Net Margin Gate). Document the contract:
- What Stage 10 hands to Stage 11 (selected_quote_id, supplier_cost, etc.)
- What Stage 11 hands to Stage 12 (landed_cost, decision_grade, blockers)

---

## PHASE 3: External Dependency Research

For each external API / data source the docs reference, document:
- The actual API URL (per documentation; do NOT call it)
- Authentication model (key, OAuth, public)
- Rate limits / costs
- Data freshness (real-time, daily, monthly)
- Whether ANY integration code exists in our repo
- The L1-L3 vs L5 use distinction

Specific APIs to research:

1. **HMRC Trade Tariff API** — duty rates, commodity codes, measures
   - URL: `https://www.trade-tariff.service.gov.uk/api/`
   - Used for: classification freshness, duty rate lookup, trade remedy detection
2. **HMRC FPO (Find Previous Operations / Tariff Classification Service)** — ML-powered commodity code suggestion
3. **HMRC monthly customs FX rates** — for customs valuation
4. **Xe.com or similar** — daily commercial FX spot rates
5. **Freightos** — international freight estimates
6. **SeaRates** — sea freight estimates
7. **TRA (Trade Remedies Authority) feed** — for trade remedy watcher

---

## PHASE 4: Synthesize the Research Document

Write the output to `docs/research/stage-11-landed-cost-research.md`. Use this structure:

```markdown
# Stage 11: Landed Cost — Research Document

> Generated: {timestamp}
> Status: Research complete — ready for build planning
> Source-of-truth hierarchy: Code > Strategy docs > Research doc

## 0. Executive Summary (TLDR)

**What Stage 11 does:** {one sentence}

**Where it stands today:** {one sentence — e.g., "schema exists, fixture-only, zero real integration code"}

**To ship screening-grade Stage 11 (L1-L3 minimum):** {N rough hours/days, M new Rust modules, K external API integrations}

**To ship decision-grade Stage 11 (L5):** {N rough hours/days, M new modules, K APIs}

**Top 3 surprises from the research:**
1. ...
2. ...
3. ...

## 0.1 Decision Points (for the user)

The following decisions need to be made before any build work begins:

| # | Decision | Options | Recommendation |
|---|----------|---------|----------------|
| 1 | Build screening-grade Stage 11 now? | Yes (unblocks Stage 12) / No (defer until more data) | ... |
| 2 | Which freight model for L1-L3? | Flat per-mode rates / single-source API / skip entirely | ... |
| 3 | Which classification source first? | HMRC FPO / CPV-priors / keyword search | ... |
| 4 | VAT modeling complexity? | Standard rate only / full rate determination | ... |
| 5 | Conservative defaults for missing data? | Apply MFN+residual+standard / require all inputs | ... |

(Add/remove rows as the research surfaces actual decisions.)

## 1. Mission and Scope

What Stage 11 does in one paragraph. The cost equation it produces. Why it sits between Stage 10 and Stage 12.

## 2. Conceptual Model

### 2.1 The cost equation
```
economic_landed_cost =
    supplier_cost
  + freight_cost
  + customs_duty
  + trade_remedy_duty
  + import_vat (recoverable portion handled separately)
  + uk_final_mile_cost
  + other_fees
```

### 2.2 Decision grade vs screening grade
Define both. Show the column-level diff.

### 2.3 Three scenarios
low / base / high — how each is computed.

### 2.4 Constitutional conservative defaults
The full list of safety defaults applied when data is missing or ambiguous.

## 3. Database Schema (`landed_cost_estimates`)

Full column-by-column documentation. Group by:
- Identity (id, FKs)
- Shipping (mode, packing, lead time, transit time)
- Classification (commodity codes, additional codes, supplementary units)
- Customs valuation (method, mode, FX rates, FX delta)
- Cost components (supplier, freight L/B/H, duty, trade remedy, VAT, final mile, other)
- Aggregates (economic landed cost, cash outlay, working capital)
- Confidence and grade (decision_grade, confidence_bps)
- Blockers and warnings

For each column, indicate:
- L0 schema (always present as nullable)
- L1-L3 populated by screening-grade flow
- L5 populated by decision-grade flow

## 4. Customs Classification

### 4.1 Multi-source recall (5 parallel sources)
Detailed walkthrough of each source, what it provides, where it lives in our codebase (or doesn't yet).

### 4.2 Multi-family expansion (HS6 → CN8 → UK 10-digit)
Why we don't greedy-descend. The `product_commodity_codes` junction.

### 4.3 Conservative defaults applied
MFN rate, residual exporter, out-of-quota, etc.

### 4.4 Classification freshness
The daily TRA watcher, `tariffCheckedAt`, when classification is considered stale.

## 5. Duty Computation

### 5.1 Duty types preserved
ad valorem, specific per unit, compound — never flattened.

### 5.2 Measure interaction
AD / CVD / safeguard / quota stacking rules.

### 5.3 Trade remedy detection
Anti-dumping cases, countervailing measures, safeguards.

### 5.4 CBAM scope
Iron / steel / aluminium / cement / fertiliser / hydrogen / electricity.

### 5.5 Customs branch divergence
The hard-blocker threshold (find exact value in docs).

### 5.6 Exporter rate basis
named_exporter / annex_exporter / residual_default — how this is determined.

## 6. FX Rate Engine

### 6.1 Dual FX model
HMRC customs rate (monthly) vs Xe commercial spot (daily). `fx_basis_delta_bps`.

### 6.2 Currency flow
USD/EUR/CNY supplier price → GBP customs valuation → GBP commercial cost.

### 6.3 L1-L3 vs L5
Single FX snapshot vs dual FX engine.

## 7. Freight Engine

### 7.1 L1-L3 screening-grade
Flat per-mode rate (parcel / pallet / container). Where the rates come from.

### 7.2 L5 decision-grade
Freightos + SeaRates API integration. Three scenarios (low/base/high).

### 7.3 Lead time / transit time
How both are estimated.

## 8. VAT Treatment

### 8.1 VAT rate determination (separate from recovery)
Standard rate, reduced rate, zero rate, exempt.

### 8.2 VAT recovery model
Recoverable vs non-recoverable for the customer.

### 8.3 National additional codes

## 9. Decision Grade Output

### 9.1 The `decision_grade` field
Values: `'review' / 'rfq_required' / 'broker_required'`. When each is set.

### 9.2 Hard blockers
What can fail Stage 11 outright.

### 9.3 Confidence aggregation
How `confidence_bps` is computed.

## 10. External Dependencies (APIs)

Detailed table of every external API:

| API | URL | Auth | Rate Limit | Cost | Used For | L1-L3 / L5 | Integration Status |
|-----|-----|------|-----------|------|----------|-----------|-------------------|
| HMRC Trade Tariff | `https://www.trade-tariff.service.gov.uk/api/` | Public | (verify) | Free | Duty rates, classification freshness, trade remedy detection | Both | Zero |
| HMRC FPO (Tariff Classification Service) | (verify URL) | (verify) | (verify) | (verify) | ML-powered commodity code suggestion | L5 | Zero |
| HMRC monthly customs FX | (verify URL) | Public | Monthly | Free | Customs valuation FX rate | Both | Zero |
| Xe / commercial FX | `api.xe.com` (or alternative) | API key | (verify) | Paid | Daily commercial spot rate | L5 | Zero |
| Freightos | (verify URL) | API key | (verify) | Paid | International freight estimates | L5 | Zero |
| SeaRates | (verify URL) | API key | (verify) | Paid | Sea freight estimates | L5 | Zero |
| TRA (Trade Remedies Authority) feed | (verify URL) | (verify) | (verify) | Free | Trade remedy daily watcher | L5 | Zero |

**Cost summary for the user's budgeting:**
- L1-L3 minimum: only HMRC Trade Tariff + HMRC FX (both free) — zero ongoing cost
- L5 decision-grade: adds Xe + Freightos + SeaRates (paid APIs) — note any pricing tiers found in the docs

## 10A. DPS / Service-Heavy Data Reality Check

The current pipeline ingests primarily ProContract data, where ~90% of qualified contracts are DPSs (Dynamic Purchasing Systems) — multi-year supplier-admission frameworks, not direct orders. Of the few non-DPS goods contracts that reach Stage 11:
- Many are services or service-heavy frameworks
- The actual product mix is light on the categories where L5 features matter most (no iron/steel/aluminium → CBAM not needed for our actual data; few low-value high-volume Chinese imports → trade remedy detection is overkill)

**Implications to surface in the research doc:**
- Which L5 features are ACTUALLY needed for our likely product mix?
- Could a pragmatic L1-L3 screening-grade landed cost (HMRC duty lookup + flat freight + standard VAT) be sufficient for current data?
- Which L5 features can safely be deferred until we ingest more direct-order goods contracts?

This is an explicit Decision Point for the user — flag any L5 feature that is theoretically powerful but unlikely to fire on current data.

## 11. Current Implementation Status

### 11.1 What exists
- Schema (L0): `landed_cost_estimates` table with ~50 columns
- Fixture-materialization in `main.rs`: hardcoded values for tests
- Downstream consumers in `packages/intelligence/`

### 11.2 What's missing for L1-L3 (screening-grade)
- HMRC Trade Tariff API client
- Commodity classification recall (any of the 5 sources)
- Flat-rate freight table
- HMRC monthly FX rate fetcher
- VAT computation
- The actual `run_landed_cost_stage()` deterministic pipeline function
- Wiring into `run_deterministic_pipeline()`

### 11.3 What's deferred to L5 (decision-grade)
- Multi-source classification recall (all 5 sources)
- Multi-family HS6/CN8/UK10 expansion
- Measure interaction engine
- Trade remedy daily watcher
- CBAM placeholder
- Dual FX engine (HMRC + Xe)
- Freightos + SeaRates freight APIs
- Exporter rate basis resolution
- Supplementary unit enforcement
- Decision-grade scenarios (low/base/high)
- Customs branch divergence detection

## 12. Build Roadmap (recommendations only — not a build plan)

### 12.1 L1-L3 minimum-viable Stage 11

For each work item: rough effort (S/M/L = small/medium/large), dependency, and what it unlocks.

| Item | Effort | Depends on | Unlocks |
|------|--------|-----------|---------|
| HMRC Trade Tariff API client (Rust) | M | None | Real duty lookup |
| Flat-rate freight table (parcel/pallet/container) | S | None | Freight estimates |
| HMRC monthly FX rate fetcher + cache | S | None | Currency conversion |
| Standard VAT computation | S | None | VAT line item |
| Commodity classification (single-source: keyword search) | M | HMRC Trade Tariff client | Duty rate lookup needs commodity code |
| `run_landed_cost_stage()` deterministic pipeline function | M | All of the above | The actual stage |
| Wiring into `run_deterministic_pipeline()` | S | run_landed_cost_stage exists | Pipeline runs end-to-end |
| Conservative defaults applied | S | All of the above | Safety when data missing |
| `decision_grade` set to `'review'` (since L1-L3 is screening-grade) | S | All of the above | Downstream stages know reliability |

**Total rough estimate (L1-L3 minimum):** ~{N} person-days, ~{M} new Rust modules.

### 12.2 L5 decision-grade upgrades

| Item | Effort | Notes |
|------|--------|-------|
| Multi-source classification recall (5 sources) | L | The big lift |
| Multi-family HS6/CN8/UK10 expansion | M | |
| Measure interaction engine | L | AD/CVD/safeguard/quota stacking |
| Trade remedy daily watcher | M | TRA feed |
| CBAM placeholder | S | Iron/steel/aluminium scope only |
| Dual FX engine (HMRC + Xe) | S | Xe API integration |
| Freightos + SeaRates freight APIs | M | Real freight estimates |
| Exporter rate basis resolution | M | Named/annex/residual logic |
| Supplementary unit enforcement | S | |
| Decision-grade scenarios (low/base/high) | M | |
| Customs branch divergence detection | S | |

**Total rough estimate (L5 decision-grade):** ~{N} person-days.

**Recommendation framework for the user:**
- If goal is "unblock Stage 12 with screening-grade data ASAP": ship 12.1 only
- If goal is "production-quality margin estimates for real bidding": need 12.2
- Hybrid: ship 12.1, defer 12.2 features until specific contracts demand them (e.g., add CBAM only when an iron/steel contract enters the pipeline)

## 13. Open Questions

Things the docs don't specify clearly, or where the docs disagree with each other, or where the codebase has diverged from the docs. Each as a numbered question for the user to resolve before build.

## 14. References

Every doc cited, every codebase file referenced, every external URL. Footnoted throughout the document.
```

---

## VERIFICATION (before submitting the research document)

The research document must pass these checks:

1. **Every section is populated** — no TODO placeholders
2. **Every claim is sourced** — either to a doc (with section number / line range) or to a codebase file (with line numbers)
3. **The L1-L3 vs L5 distinction is crystal clear** for every feature
4. **External APIs are listed with URLs and current integration status** — so the user knows what's free, what's paid, what's available
5. **Open questions are explicit** — don't paper over ambiguity; surface it
6. **The cost equation is shown algebraically** with each term explained
7. **The schema documentation is complete** — every nullable column accounted for
8. **The conservative defaults are listed** — the full set, not summarized
9. **The classification recall is fully described** — all 5 sources with their roles
10. **The duty computation supports compound types** — not just ad valorem percentages

---

## DELIVERABLES

1. **`docs/research/stage-11-landed-cost-research.md`** — the comprehensive research document (target: 800-1200 lines; if you're at <600 you've under-researched, if you're at >1400 you've over-included tangential content)
2. **A short summary** at the top of your response to the user with:
   - Total documentation files inspected (use grep counts as proxy)
   - Total codebase files inspected
   - The Executive Summary's "Top 3 surprises" preview
   - The Decision Points table preview (just the decisions, not the analysis)
   - Top 3-5 open questions

---

## CONSTRAINTS

1. **NO code changes.** No edits to `crates/`, `packages/`, `skills/`, `migrations/`, or any source file.
2. **NO database mutations.** Read-only SQLite queries only.
3. **NO external API calls.** Document the APIs from documentation; do not actually call HMRC / Xe / Freightos.
4. **Source every claim.** No assertions without a reference. Training-data assumptions about HMRC or duty rates are not acceptable.
5. **Distinguish layers explicitly.** Every feature must be tagged L1-L3, L5, or L6.
6. **Surface contradictions.** If the docs disagree with the code, or two docs disagree, document the contradiction in the Open Questions section.

---

## REMINDERS

- ULTRATHINK and be meticulous.
- This is **research-only** — the output is a document, not running code.
- The user's actual goal is a CLEAR PICTURE before deciding whether to build screening-grade Stage 11 in L1-L3 scope, or skip ahead to other stages, or defer Stage 11 entirely. The research document needs to support that decision.
- The "data reality" of our current pipeline matters too — only 1 candidate has dual-source pricing (DGU). Stage 11 may have very few candidates to operate on initially. Mention this if relevant.
- The L0 doc is the canonical source for schema. The L5 doc is the canonical source for decision-grade upgrades. The L1 doc defines screening-grade. Read all three carefully.
- If you find that significant Stage 11 code already exists (unlikely but possible — e.g., scaffolded HMRC client), document it precisely. The user's belief is "fixture-only, no real integration" — verify or correct that.
