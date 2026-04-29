# Stage 9: Marketplace Search — Build 5 Source Skills, Gather Pricing Data

ULTRATHINK and be meticulous. This is the most complex pipeline stage — 5 separate agent skills, each scraping a different marketplace, with configurable pricing parameters.

## FRESH-EYES CURRENT-STATE OVERRIDE (2026-04-26)

This handoff was originally written as a build prompt. **Do not recreate the Stage 9 skill packages or daemon wiring if they still exist.** Current code already has the five marketplace sources, `persist_marketplace_search_results()`, `load_candidates_for_marketplace_search()`, `run_marketplace_search_live_loop()`, and source dispatch for `gem-imports`, `mx-wholesale`, `amazon-uk`, `ebay-uk`, and `alibaba-search` in `crates/daemon/src/main.rs`.

Current read-only DB snapshot:

```text
search_profiles=820
search_attempts=193
supplier_quotes=10
quotes by source_name: alibaba-search=2, amazon-uk=3, ebay-uk=5
quotes by price_signal_role: cn_supplier_floor=2, uk_retail_benchmark=8
```

Use this handoff as architecture/source-contract context. The current mission, if handed to a fresh agent, is **Stage 9 validation/repair/extension**, not greenfield build: verify the existing five-source machinery, improve live-run coverage or source playbooks only where current evidence shows failure, and preserve the raw-price/price-signal-role contract for Stage 10.

## MISSION

Validate, repair, and extend the existing **Stage 9 marketplace search** system: five marketplace agent skills plus daemon dispatch/persistence that find supplier prices for searchable product candidates. The five-source infrastructure is already built; a receiving agent should first verify current DB/code state, then improve live-run coverage, source playbooks, or persistence defects only where evidence shows a gap. The output Stage 9 contract is `search_attempts` + `supplier_quotes` with raw prices preserved and `price_signal_role` set for Stage 10 margin math.

**The 5 sources (in search priority order):**

| Priority | Source | URL | Role | Region |
|----------|--------|-----|------|--------|
| Primary UK | **Gem Imports** | `https://www.gemimports.co.uk/` | UK wholesale/trade supplier | UK |
| Primary UK | **MX Wholesale** | `https://www.mxwholesale.co.uk/` | UK wholesale supplier | UK |
| Secondary UK | **Amazon UK** | `https://www.amazon.co.uk/` | UK retail benchmark (ceiling) | UK |
| Secondary UK | **eBay UK** | `https://www.ebay.co.uk/` | UK retail benchmark (ceiling) | UK |
| Chinese | **Alibaba** | `https://www.alibaba.com/` | Chinese supplier floor price | CN |

**Search order:** Try primary UK sources first (Gem Imports, MX Wholesale), then secondary UK (Amazon, eBay), then Chinese (Alibaba). The goal is to find each product across as many sources as possible.

**Pricing model:**
- UK retail prices (Amazon, eBay) are consumer prices — apply a **configurable wholesale discount** (default 30%, range 20-40%) to estimate wholesale cost
- Gem Imports and MX Wholesale may show wholesale/trade prices directly — no discount needed if prices are already trade
- Alibaba shows price ranges with bulk tiers — extract ALL tiers (MOQ, price per tier)
- Amazon/eBay: the agent must intelligently look for bulk/multi-pack options where the per-unit price is lower

**After Stage 9 is healthy we should be able to:** feed Stage 10 with supplier quotes from UK retail/wholesale benchmarks and Chinese supplier-floor sources, including enough dual-source pairs to estimate raw margin.

```
┌─────────────────────────────────────────────────────────┐
│                    THE LOOP                               │
│                                                          │
│   FOR EACH of the 5 sources:                             │
│     1. Explore the website with agent-browser             │
│     2. Build the skill package (SKILL.md + schemas)       │
│     3. Test with sub-agent on 3-5 products                │
│     4. Validate result data                               │
│     5. Fix issues → repeat until skill works              │
│                                                          │
│   THEN:                                                   │
│     6. Wire all 5 into daemon                             │
│     7. Run on the current searchable candidate set         │
│     8. Validate full data set                             │
│     9. Write manifest                                    │
│                                                          │
│   DO NOT STOP UNTIL ALL 5 SKILLS GREEN                   │
└─────────────────────────────────────────────────────────┘
```

---

## CONTEXT: What Feeds Stage 9

```
From Stage 8 (GREEN):
  545 product_candidates scored (quoteability_score_bps populated)
  545 search_profiles with query_waves_json
  Historical baseline: 57 searchable candidates in passed-goods (23 eligible + 34 marginal)
  Current runs must query the live DB first; the 2026-04-26 snapshot has 820
  search_profiles, with eligible=414, marginal=325, ineligible=81.

Stage 9 reads:
  search_profiles.query_waves_json → search terms per candidate
  search_profiles.search_eligibility_state → eligible/marginal/ineligible
  product_candidates.head_noun, name, specification_text → context for matching

Stage 9 writes:
  search_attempts → one row per (candidate × source × wave) recording what was searched
  supplier_quotes → one row per usable quote found
  listing_evidence → cached marketplace listing data (optional)
```

---

## PHASE 0: CODEBASE RESEARCH (mandatory before building any skill)

### 0A. Read AGENTS.md — single source of truth for agent instructions.

### 0B. Understand Skill Execution

Search `crates/daemon/src/main.rs` for `execute_skill_session` — this is how the daemon spawns Claude CLI in PTY mode to run a skill. The skill receives `job_parameters` as JSON and writes `result.json` to its working directory.

Study the existing skill packages at `skills/procurement-ingestion/contracts-finder/` and `skills/procurement-ingestion/procontract-discover/` as reference patterns for SKILL.md structure, result.schema.json, and the 5-file package layout.

### 0C. Understand the Database Tables You'll Write To

**`search_attempts` table** (one row per search query executed):
```sql
CREATE TABLE search_attempts (
  id TEXT PRIMARY KEY,
  product_candidate_id TEXT NOT NULL REFERENCES product_candidates(id),
  search_profile_id TEXT NOT NULL REFERENCES search_profiles(id),
  source TEXT NOT NULL,          -- e.g., "gem-imports", "alibaba"
  locale TEXT NOT NULL,          -- e.g., "en-GB", "en-US"
  wave_number INTEGER NOT NULL,  -- which query wave (1-4)
  query_text TEXT NOT NULL,      -- actual search text used
  filters_json TEXT,             -- any filters applied
  is_recovery_wave INTEGER NOT NULL DEFAULT 0,
  result_count INTEGER NOT NULL DEFAULT 0,     -- total results on page
  hard_rejected_count INTEGER NOT NULL DEFAULT 0,
  shortlisted_count INTEGER NOT NULL DEFAULT 0, -- results worth quoting
  search_failure_mode TEXT,      -- null if successful
  session_id TEXT,
  executed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  owner_id TEXT NOT NULL, row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER, stage_run_id TEXT
);
```

**`supplier_quotes` table** (one row per usable price found) — key columns:
```sql
CREATE TABLE supplier_quotes (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL,
  notice_id TEXT NOT NULL,
  product_candidate_id TEXT REFERENCES product_candidates(id),
  search_attempt_id TEXT REFERENCES search_attempts(id),
  product_name TEXT NOT NULL,        -- listing title from marketplace
  region TEXT NOT NULL,              -- "GB" or "CN"
  quote_class TEXT NOT NULL,         -- "spot" / "wholesale" / "trade"
  source_name TEXT NOT NULL,         -- "gem-imports", "amazon-uk", "alibaba"
  source_url TEXT,                   -- link to the listing
  source_listing_id TEXT,            -- marketplace's product ID
  unit_price_original_minor INTEGER NOT NULL,  -- raw price in source currency minor units
  currency TEXT NOT NULL,            -- "GBP" or "USD"
  unit_price_gbp_pence INTEGER NOT NULL,       -- converted to GBP pence
  minimum_order_quantity NUMERIC,
  price_basis_qty NUMERIC,           -- quantity the price applies to
  price_basis_unit TEXT,
  includes_shipping INTEGER NOT NULL DEFAULT 0,
  price_signal_role TEXT NOT NULL,   -- "uk_wholesale" / "uk_retail_benchmark" / "cn_supplier_floor"
  vat_inclusive INTEGER,
  match_confidence_bps INTEGER,      -- how well does this match the candidate?
  quote_usability_confidence_bps INTEGER,
  quote_realism_confidence_bps INTEGER,
  evidence TEXT NOT NULL,            -- why this quote matches
  owner_id TEXT NOT NULL, row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER, stage_run_id TEXT
);
```

### 0D. Result JSON → DB Column Mapping

Your skill's result.json `quotes[]` map to `supplier_quotes` columns like this:

| Result JSON Field | DB Column | Notes |
|---|---|---|
| `product_name` | `product_name` | Listing title |
| `source_url` | `source_url` | Product page URL |
| `source_listing_id` | `source_listing_id` | Marketplace product ID |
| `unit_price_minor` | `unit_price_original_minor` | Raw price in source currency minor units (pence for GBP, cents for USD) |
| `currency` | `currency` | "GBP" or "USD" |
| (converted) | `unit_price_gbp_pence` | Daemon converts USD→GBP |
| `vat_inclusive` | `vat_inclusive` | 1 or 0 |
| `minimum_order_quantity` | `minimum_order_quantity` | MOQ if shown |
| `match_confidence_bps` | `match_confidence_bps` | 0-10000 |
| `evidence` | `evidence` | Text explanation of match |
| `ship_from_country` | `ship_from_country` | "GB" or "CN" |
| `includes_shipping` | `includes_shipping` | 1 or 0 |
| (from job_parameters) | `process_id`, `notice_id`, `product_candidate_id` | Daemon fills from context |
| (per source) | `price_signal_role` | Daemon sets based on source |
| (per source) | `source_name` | Daemon sets: "gem-imports", etc. |
| (per source) | `region` | Daemon sets: "GB" or "CN" |

### 0E. Sample Products to Search For

These are the actual top eligible candidates the agent will be searching for:

| Name | Head Noun | Contract |
|---|---|---|
| Double-glazed unit (sealed insulating glass unit, BS EN 1279-compliant) | double-glazed | BSH Double Glazed Units DPS |
| Metal railings / gates / fencing — repair and modification | metal | BSH Metal Works DPS |
| Manufactured metal railings — supply and installation | manufactured | BSH Metal Works DPS |
| Standard-run brochures / leaflets / flyers (A4, A5) | standard-run | CCP074 Print Service DPS |
| Envelopes C4 / C5 / DL (pocket, wallet, window) | envelopes | CCP074 Print Service DPS |
| Softwood timbers — general construction grade | softwood | BSH Joinery Timber DPS |
| Plywood and door blanks | plywood | BSH Joinery Timber DPS |

You must be able to answer before building any skill:
1. What tables does Stage 9 write to? (`search_attempts` + `supplier_quotes`)
2. What is the difference between `unit_price_original_minor` and `unit_price_gbp_pence`? (original = source currency, gbp = converted)
3. How does the skill receive the list of products to search? (via `job_parameters`)
4. What does the skill do when a search returns zero results? (records a `search_attempts` row with `result_count=0`, no `supplier_quotes` created, moves to next wave)

---

## KEY CONCEPTS

**Price-signal roles:** Each source provides a different kind of price signal:

| Source | Price Signal Role | What It Means |
|--------|------------------|---------------|
| Gem Imports | `uk_wholesale` | Trade/wholesale price — closest to actual procurement cost |
| MX Wholesale | `uk_wholesale` | Trade/wholesale price |
| Amazon UK | `uk_retail_benchmark` | Consumer price ceiling — apply wholesale discount |
| eBay UK | `uk_retail_benchmark` | Consumer price ceiling — apply wholesale discount |
| Alibaba | `cn_supplier_floor` | Chinese wholesale floor — optimistic sourcing cost |

**Wholesale discount parameter:** For UK retail sources (Amazon, eBay), store the RAW retail price in `unit_price_original_minor` and the discount-adjusted estimate in a separate field or via `negotiation_uplift_bps`. The discount percentage should be configurable — store it as a constant in the daemon that can be changed without modifying the data. **Default: 3000 bps (30% discount).** This means: estimated wholesale = retail price × 0.70.

**Why we record everything:** Even if a source shows consumer prices, we record the raw price AND the estimated wholesale. The raw data is permanent; the discount parameter can be tweaked later without re-scraping.

---

## ARCHITECTURE: 5 Skills

Each marketplace source gets its own skill package under `skills/marketplace-search/`:

```
skills/marketplace-search/
  gem-imports/          SKILL.md, result.schema.json, checkpoint.schema.json,
  mx-wholesale/           canary.inputs.jsonl, golden.assertions.yaml
  amazon-uk/
  ebay-uk/
  alibaba/
```

All 5 skills share the SAME result contract schema (different from the ingestion schema). The daemon runs them sequentially per candidate: primary UK → secondary UK → Chinese.

### Shared Result Contract Shape

Each skill receives a list of products to search for (via job_parameters) and returns quotes found:

```json
{
  "source": "gem-imports",
  "cursor": "{}",
  "continuation_required": false,
  "search_results": [
    {
      "product_candidate_id": "pc-xxx-candidate-yyy",
      "search_profile_id": "sp-pc-xxx-candidate-yyy",
      "query_used": "double glazed units 4mm toughened",
      "wave_number": 1,
      "result_count": 5,
      "shortlisted_count": 2,
      "quotes": [
        {
          "product_name": "4mm Toughened Double Glazed Unit 600x900mm",
          "source_url": "https://www.gemimports.co.uk/product/...",
          "source_listing_id": "GEM-12345",
          "unit_price_minor": 2500,
          "currency": "GBP",
          "vat_inclusive": true,
          "quantity": 1,
          "quantity_unit": "unit",
          "minimum_order_quantity": null,
          "price_tiers": [
            {"min_qty": 1, "unit_price_minor": 2500},
            {"min_qty": 10, "unit_price_minor": 2200},
            {"min_qty": 50, "unit_price_minor": 1900}
          ],
          "ship_from_country": "GB",
          "includes_shipping": false,
          "is_used_or_refurbished": false,
          "match_confidence_bps": 7500,
          "evidence": "Title match: '4mm toughened double glazed unit'. Size matches spec."
        }
      ]
    }
  ]
}
```

**For Alibaba specifically**, the `price_tiers` field is critical — Alibaba shows tiered pricing (prices in USD cents, not pence):
```json
"price_tiers": [
  {"min_qty": 1, "unit_price_minor": 800, "currency": "USD"},
  {"min_qty": 100, "unit_price_minor": 600, "currency": "USD"},
  {"min_qty": 1000, "unit_price_minor": 450, "currency": "USD"}
]
```

**How price tiers map to `supplier_quotes`:** The `supplier_quotes` table does NOT have a `price_tiers` column. Instead:
- Create ONE `supplier_quotes` row using the **lowest available tier price** as `unit_price_original_minor` (most optimistic sourcing cost)
- Store the FULL tiers array as JSON in the `evidence` field alongside the match rationale
- Set `minimum_order_quantity` to the MOQ of the lowest tier
- Set `price_basis_qty` to the quantity threshold of the tier used

This way the best-case sourcing price flows into margin calculation, and the full tier data is preserved in evidence for manual review.

---

## SKILL BUILD SEQUENCE

Build one skill at a time. For each:

### Step 1: Portal Exploration

```bash
agent-browser open "{source_url}"
agent-browser wait
agent-browser snapshot -i
```

**Discovery checklist per source:**
- [ ] How does search work? (search bar, filters, categories)
- [ ] What does a search results page look like? (grid, list, pagination)
- [ ] What data is visible on listing pages vs detail pages?
- [ ] Are prices shown? VAT inclusive/exclusive?
- [ ] Are there bulk/trade pricing tiers?
- [ ] Does the site require login for trade prices?
- [ ] What's the URL pattern for product pages?
- [ ] Any anti-bot measures, CAPTCHAs, rate limiting?

### Step 2: Build Skill Package

Create `skills/marketplace-search/{source}/` with 5 files.

**SKILL.md frontmatter:**
```yaml
---
skill_id: marketplace-search/{source}
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
maxPages: 10
maxScrollCycles: 3
---
```

**SKILL.md playbook must include:**
- Step-by-step instructions for the agent to:
  1. Read the product list from job_parameters
  2. Open the marketplace website
  3. For each product: search using the query waves from the search profile
  4. Browse search results, identify matching products
  5. Visit detail pages to extract full pricing data
  6. Extract: product name, price, price tiers (if available), MOQ, shipping, pack sizes
  7. Assess match confidence (does the product match the specification?)
  8. Write result.json to the working directory

### Step 3: Test with Sub-Agent

Spawn a sub-agent with realistic job_parameters matching what the daemon will provide:

```
You are a marketplace search agent. Read skills/marketplace-search/{source}/SKILL.md
and execute the playbook using agent-browser. Your job_parameters are:

{
  "source": "{source}",
  "candidates": [
    {
      "product_candidate_id": "pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-dgu-sealed",
      "search_profile_id": "sp-pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-dgu-sealed",
      "name": "Double-glazed unit (sealed insulating glass unit, BS EN 1279-compliant)",
      "head_noun": "double-glazed",
      "specification_text": "Flat hermetically-sealed double-glazed units...",
      "query_waves": [
        {"wave": 1, "strategy": "exact_noun", "query": "double glazed units 4mm toughened sealed"},
        {"wave": 2, "strategy": "noun_with_spec", "query": "double glazed units BS EN 1279"},
        {"wave": 3, "strategy": "broader_noun", "query": "sealed glazing units"},
        {"wave": 4, "strategy": "existence_check", "query": "double glazed units"}
      ]
    },
    {
      "product_candidate_id": "pc-b7aaab27-1f34-ed11-8119-005056b64545-lot-1-candidate-softwood",
      "search_profile_id": "sp-pc-b7aaab27-1f34-ed11-8119-005056b64545-lot-1-candidate-softwood",
      "name": "Softwood timbers — general construction grade",
      "head_noun": "softwood",
      "specification_text": "Supply and delivery of softwood timbers for construction...",
      "query_waves": [
        {"wave": 1, "strategy": "exact_noun", "query": "softwood timber construction grade"},
        {"wave": 2, "strategy": "noun_with_spec", "query": "softwood treated timber"},
        {"wave": 3, "strategy": "broader_noun", "query": "construction timber"},
        {"wave": 4, "strategy": "existence_check", "query": "softwood timber"}
      ]
    }
  ]
}

Write result.json to the working directory with quotes found.
```

This gives the agent real product data and realistic query waves to work with during testing.

### Step 4: Validate Results

Check result.json:
- Valid JSON matching the result schema
- At least 1 quote found for at least 1 product (some products may have 0 — that's OK)
- Prices are integers in pence (not pounds, not floats)
- Match confidence scores are reasonable (0-10000 bps)
- Source URLs are real, navigable product pages
- For Alibaba: `price_tiers` array has at least 1 entry per quote
- `currency` is correct ("GBP" for UK sources, "USD" for Alibaba)

### Handling Zero Results

When a product search returns no matching results on a particular source:
- The skill MUST still include that product in `search_results` with `result_count: 0` and empty `quotes: []`
- This ensures the daemon creates a `search_attempts` row recording that the search was made (important for coverage tracking)
- The skill should try ALL query waves before giving up — wave 4 (broadest) may find something waves 1-3 missed
- If ALL waves return zero: `search_failure_mode: "no_results_all_waves"` in the result

---

## SOURCE-SPECIFIC GUIDANCE

### Source 1: Gem Imports (`gemimports.co.uk`)

- UK wholesale/trade supplier
- May show trade prices directly (no discount needed)
- Check if registration/login required for trade prices
- Products likely organized by category
- Expected product types: hardware, building materials, consumables
- `price_signal_role = "uk_wholesale"`
- `vat_inclusive`: check — trade suppliers often show ex-VAT

### Source 2: MX Wholesale (`mxwholesale.co.uk`)

- UK wholesale supplier
- Similar to Gem Imports — trade prices likely shown
- Check for bulk pricing tiers
- `price_signal_role = "uk_wholesale"`
- May have MOQ requirements

### Source 3: Amazon UK (`amazon.co.uk`)

- Consumer retail marketplace — prices are retail
- **Apply wholesale discount:** store raw retail price, note `price_signal_role = "uk_retail_benchmark"`
- The agent must look for:
  - Multi-pack options (e.g., "Pack of 10" at lower per-unit price)
  - "Subscribe & Save" pricing
  - Different sellers offering the same product at different prices
  - Bulk/trade listings (Amazon Business)
- Extract the BEST per-unit price across all options
- No login required

### Source 4: eBay UK (`ebay.co.uk`)

- Consumer marketplace — mix of retail and trade sellers
- **Apply wholesale discount** for retail listings
- The agent must:
  - Filter for "Buy It Now" (not auctions)
  - Filter for "New" condition
  - Look for trade/wholesale sellers (higher quantities, lower per-unit)
  - Check multi-item listings
- `price_signal_role = "uk_retail_benchmark"`

### Source 5: Alibaba (`alibaba.com`)

- Chinese B2B marketplace — wholesale/factory prices
- **No wholesale discount needed** — prices ARE wholesale
- `price_signal_role = "cn_supplier_floor"`
- `ship_from_country = "CN"` (or specific country shown)
- The agent MUST extract:
  - **All price tiers** (Alibaba shows: "1-99 units: $X.XX, 100-999: $Y.YY, 1000+: $Z.ZZ")
  - MOQ (minimum order quantity)
  - Shipping: FOB/CIF/EXW terms
  - Supplier location
  - "Ready to Ship" status (boosts usability confidence)
  - Trade Assurance status
- Prices are in USD — convert to GBP pence using current rate
- Currency conversion: store original USD price in `unit_price_original_minor` AND converted GBP in `unit_price_gbp_pence`
- Use current approximate rate: 1 USD ≈ 79 GBP pence (the daemon can apply exact FX later)

---

## WHOLESALE DISCOUNT CONFIGURATION

Add a configurable wholesale discount constant to the daemon. This is NOT hardcoded per quote — it's a system-level parameter applied during margin calculation:

```rust
/// Default wholesale discount for UK retail benchmark prices.
/// 3000 bps = 30% discount (wholesale ≈ 70% of retail).
/// Adjustable range: 2000-4000 bps (20-40%).
const DEFAULT_UK_RETAIL_WHOLESALE_DISCOUNT_BPS: i64 = 3000;
```

**How it works:**
- Raw retail price is stored in `supplier_quotes.unit_price_gbp_pence` as-is
- The `price_signal_role` field marks it as `uk_retail_benchmark`
- When computing margins (Stage 10+), the daemon applies the discount: `estimated_wholesale = retail * (10000 - discount_bps) / 10000`
- Changing the constant recalculates margins without re-scraping

For wholesale sources (Gem Imports, MX Wholesale, Alibaba), no discount is applied — prices are already wholesale/trade.

---

## DAEMON INTEGRATION

### Source Registry

Add 5 source entries to `builtin_definitions()` in `registry.rs`:

```rust
// marketplace-search sources (Stage 9)
"gem-imports"    → marketplace-search/gem-imports,    cron: None (manual trigger for now)
"mx-wholesale"   → marketplace-search/mx-wholesale,   cron: None
"amazon-uk"      → marketplace-search/amazon-uk,      cron: None
"ebay-uk"        → marketplace-search/ebay-uk,        cron: None
"alibaba-search" → marketplace-search/alibaba,        cron: None
```

### Live Loop / Job Dispatch

Stage 9 is different from ingestion — it's candidate-driven, not source-driven. The daemon needs to:

1. Query `search_profiles` for eligible/marginal candidates
2. For each candidate, create search jobs for each marketplace source
3. Each job receives: `product_candidate_id`, `search_profile_id`, `query_waves_json`, `candidate_name`, `head_noun`, `specification_text`
4. The skill searches that specific product on its marketplace
5. Results are persisted to `search_attempts` + `supplier_quotes`

**Custom persist function needed:** `persist_marketplace_search_results()` that reads the skill's result contract and writes:
- `search_attempts` — one row per (candidate × source × wave) searched
- `supplier_quotes` — one row per usable quote found
- `listing_evidence` — cached listing data (optional)
- `persist_stage()` for `"Price-signal match"` with scope_kind `"candidate_source_pair"`

---

## VALIDATION

After all 5 skills are built and run on the 57 candidates:

### Check 9.1: Search attempts recorded
```bash
sqlite3 state/state.db "
SELECT source, COUNT(*) FROM search_attempts GROUP BY source ORDER BY COUNT(*) DESC;
"
```
**PASS:** At least 3 sources have attempts. Total attempts > 100.

### Check 9.2: Supplier quotes found
```bash
sqlite3 state/state.db "
SELECT source_name, price_signal_role, COUNT(*), AVG(unit_price_gbp_pence) as avg_price_pence
FROM supplier_quotes GROUP BY source_name, price_signal_role;
"
```
**PASS:** At least 2 sources have quotes. Alibaba prices should be lower than UK prices.

### Check 9.3: Price signal roles correct
```bash
sqlite3 state/state.db "
SELECT price_signal_role, COUNT(*) FROM supplier_quotes GROUP BY price_signal_role;
"
```
**PASS:** Both `uk_wholesale`/`uk_retail_benchmark` AND `cn_supplier_floor` present.

### Check 9.4: Passed-goods candidates with quotes
```bash
sqlite3 state/state.db "
SELECT 'Candidates with >=1 quote: ' || COUNT(DISTINCT product_candidate_id) FROM supplier_quotes sq
  JOIN product_candidates pc ON pc.id = sq.product_candidate_id
  JOIN qualification_runs qr ON qr.process_id = pc.process_id
  WHERE qr.qualification_basis = 'notice_only' AND qr.goods_qualified = 1 AND qr.expiry_qualified = 1;
"
```
**PASS:** At least 10 currently searchable candidates have quotes, or the manifest explains why the current candidate set cannot support that threshold.

### Check 9.5: Raw margin estimate possible
```bash
sqlite3 state/state.db "
SELECT pc.name,
  MIN(CASE WHEN sq.price_signal_role IN ('uk_wholesale', 'uk_retail_benchmark') THEN sq.unit_price_gbp_pence END) as uk_price_pence,
  MIN(CASE WHEN sq.price_signal_role = 'cn_supplier_floor' THEN sq.unit_price_gbp_pence END) as cn_price_pence
FROM supplier_quotes sq
JOIN product_candidates pc ON pc.id = sq.product_candidate_id
GROUP BY pc.id
HAVING uk_price_pence IS NOT NULL AND cn_price_pence IS NOT NULL
LIMIT 10;
"
```
**PASS:** At least 1 candidate has both UK and CN prices — raw margin can be estimated.

### Check 9.6: FK integrity
```bash
sqlite3 state/state.db "
SELECT 'Orphan quotes (candidate): ' || COUNT(*) FROM supplier_quotes sq
  LEFT JOIN product_candidates pc ON pc.id = sq.product_candidate_id WHERE pc.id IS NULL;
SELECT 'Orphan attempts (candidate): ' || COUNT(*) FROM search_attempts sa
  LEFT JOIN product_candidates pc ON pc.id = sa.product_candidate_id WHERE pc.id IS NULL;
"
```
**PASS:** Both 0.

---

## DELIVERABLES

1. **5 skill packages** under `skills/marketplace-search/` — each with SKILL.md, result.schema.json, checkpoint.schema.json, canary.inputs.jsonl, golden.assertions.yaml
2. **Daemon integration** — source registry entries, live loop/dispatch functions, custom persist function
3. **Wholesale discount constant** — configurable in daemon
4. **All 6 validation checks pass**
5. **`cargo check && cargo clippy`** clean

### MANDATORY: Write Results Manifest

After all checks pass, write to `docs/pipeline/stage-9-validation-manifest.md`:

```markdown
# Stage 9 Validation Manifest

> Generated: {timestamp}
> Status: {GREEN / YELLOW / RED}

## Source Coverage

| Source | Skill Built | Attempts | Quotes Found | Avg Price (pence) |
|--------|-----------|----------|-------------|-------------------|
| gem-imports | YES/NO | X | X | X |
| mx-wholesale | YES/NO | X | X | X |
| amazon-uk | YES/NO | X | X | X |
| ebay-uk | YES/NO | X | X | X |
| alibaba | YES/NO | X | X | X |

## Quote Distribution

| Price Signal Role | Count | Avg Price (pence) |
|------------------|-------|-------------------|
| uk_wholesale | X | X |
| uk_retail_benchmark | X | X |
| cn_supplier_floor | X | X |

## Margin Estimates (candidates with both UK + CN prices)

| Candidate | UK Best Price | CN Best Price | Raw Margin % |
|-----------|-------------|--------------|-------------|
| ... | ... | ... | ... |

## Key Metric
**Candidates with dual-source pricing (UK + CN): X of current searchable candidates (X%)**
**Average raw margin: X%**

## Validation Checks
| # | Check | Result | Detail |
|---|-------|--------|--------|
| 9.1-9.6 | ... | ... | ... |

## Issues Found & Fixed
{...}

## Data Snapshot
{Full query output}
```

---

## CONSTRAINTS

1. **Work in a loop.** Build one skill at a time, test, fix, repeat. Don't try to build all 5 simultaneously.
2. **Build order:** Gem Imports → MX Wholesale → Alibaba → Amazon → eBay. Start with wholesale sources (simpler pricing) before retail (more complex bulk logic).
3. **Each skill is `pty_interactive`** — requires agent-browser for web scraping.
4. **Store RAW prices.** Never apply the wholesale discount to stored data. Store the retail price as-is; the discount is a downstream computation parameter.
5. **Prices in pence (integers).** Convert pounds to pence (×100). Convert USD to GBP pence for Alibaba (×79 approximate, daemon applies exact FX later).
6. **`result.json` must be written to the working directory.** Same pattern as all other skills.
7. **Rust conventions:** `cargo check --all-targets && cargo clippy --all-targets -- -D warnings`.
8. **Do NOT build Stages 10-12.** Stage 9 ends at supplier_quotes. Margin computation is separate.

---

## REMINDERS

- ULTRATHINK and be meticulous.
- **The loop is non-negotiable.** Each skill must be tested with real agent-browser execution before moving to the next.
- Not every product will be found on every source. That's expected. Some products (double-glazed units, timber) will have good coverage; others (specialized print services) may have poor coverage.
- Alibaba is the most important source — it gives the Chinese wholesale price that forms the basis of margin calculation.
- Amazon and eBay are noisy — the agent needs intelligence to find the right product among thousands of irrelevant results. Look for the BEST per-unit price, not just the first result.
- The wholesale discount (default 30%) is an ESTIMATE. The user wants to be able to tweak it later. Store it as a constant, not hardcoded in SQL.
- **Current searchable candidates × 5 sources can mean hundreds of search sessions.** Each is a Claude CLI PTY session with agent-browser. This will take significant time and API usage. Query the live DB first, run a small high-confidence validation batch, then expand only if the quote yield justifies it.
