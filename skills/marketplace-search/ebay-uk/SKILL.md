---
skill_id: marketplace-search/ebay-uk
skill_version: 1.1.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
maxPages: 10
maxScrollCycles: 3
---

# marketplace-search/ebay-uk

## Use when
Use this skill when the daemon needs a UK retail benchmark ceiling from
eBay UK (`https://www.ebay.co.uk/`) for `product_candidates` whose
`search_eligibility_state` is `eligible` or `marginal`.

## Do not use when
Do not use for wholesale, Chinese floor, or ingestion.

## Inputs
- `source` — always `"ebay-uk"`.
- `candidates[]` — see the **Gem Imports SKILL.md** for the full field map.
  Beyond the basic identity fields (`product_candidate_id`,
  `search_profile_id`, `name`, `head_noun`, `specification_text`,
  `query_waves[]`), the daemon also sends structured anchors (`standards[]`,
  `critical_attributes`, `numeric_specs`, `material`, `quantity`,
  `quantity_unit`) plus parent context (`lot_title`, `lot_description`,
  `notice_title`, `notice_summary`, `buyer_name`, `cpv_codes`).

### Using the full candidate context when scoring (non-negotiable)

eBay is heterogeneous — the same head-noun surfaces domestic-DIY,
trade-seller, and retail-refurbish listings. Disambiguate via:
1. **`specification_text`** for size / grade / finish match signals, AND
2. **`standards`** if present — listings citing the same BS/EN/ISO code
   score materially higher, AND
3. **`cpv_codes`** + **`lot_title`** to validate the candidate lot-scope
   against eBay's own category. A listing whose eBay category is
   *"Home, Furniture & DIY > Windows"* is not the same as a tender for
   CPV `44221111-6 - Double-glazed units` (a trade/construction
   classification).

Record these cross-references in each quote's `evidence` field.

## Outputs
- `result.json` matching `result.schema.json`. Every candidate must
  appear in `search_results` at least once.

Prices captured are consumer retail (mixed with some trade sellers).
The daemon applies a configurable wholesale discount (default 30%)
downstream; the skill MUST store the raw listing price unchanged.

## Non-goals
- Logging in, bidding on auctions, messaging sellers.
- Converting retail → estimated wholesale.

## Failure Codes
- SOURCE_CHANGED
- SOURCE_SOFT_BLOCKED
- SOURCE_CAPTCHA_BLOCKED
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- AGENT_STALLED

## Pagination Bounds
- `maxPages`: 10
- `maxScrollCycles`: 3

---

## Playbook

You are a retail-benchmark price-search agent working on eBay UK.
eBay is a mixed marketplace: consumers, dropshippers, and trade
sellers. Your job is to find the **best plausible fixed-price listing**
for each candidate.

### Step 0: Read Job Parameters

Parse `candidates` and `query_waves`.

### Step 1: Open eBay UK

```
agent-browser open "https://www.ebay.co.uk/"
agent-browser wait 3000
agent-browser snapshot
```

Dismiss cookie banners. Do NOT log in.

### Step 2: For Each Candidate, Walk Query Waves

For each candidate, iterate `query_waves` and stop at the first wave
that produces ≥1 shortlisted quote with `match_confidence_bps >= 4000`.

#### 2a. Execute the search — filtered for Buy-It-Now + New

eBay supports URL-filtered searches. Always append these filters:

- `LH_BIN=1` (Buy It Now only — **exclude auctions**)
- `LH_ItemCondition=1000|1500` (New or New Other only — exclude used)

```
agent-browser open "https://www.ebay.co.uk/sch/i.html?_nkw=<URL-encoded query>&LH_BIN=1&LH_ItemCondition=1000%7C1500&_sop=15"
agent-browser wait 3000
agent-browser snapshot
```

`_sop=15` sorts by "Price + postage: lowest first", which aligns with
finding the best per-unit deal.

#### 2b. Shortlist up to 3 listings — prefer multi-item & trade sellers

Scan results and prefer:

- Multi-item listings ("10 pack", "Lot of 50", "Bulk 100") — divide
  total price by item count for the per-unit price.
- Listings with "Business seller" badge.
- Listings with high "items sold" counts — a weak signal of legitimate
  trade presence.

Reject:

- Auction-style listings (should already be filtered, but verify).
- Used / refurbished (verify the New filter held).
- Listings whose seller feedback is below 98% (hover to read the
  seller rating; skip if visibly low).
- Listings whose title clearly mismatches.

#### 2c. Open each shortlisted detail page

```
agent-browser click "@<ref of listing title>"
agent-browser wait 3000
agent-browser snapshot
```

Scroll to reveal variation selector / quantity picker:

```
agent-browser scroll down 600
agent-browser snapshot
```

If the listing has **variations** (e.g. "Colour: White / Size: A4")
that change the price, stay on the base/default variation — do not
click-through variations.

#### 2d. Extract raw fields

| Field | Notes |
|-------|-------|
| `product_name` | listing title |
| `source_url` | current URL (strip query params except item id) |
| `source_listing_id` | item id (e.g. `/itm/395123456789` → `395123456789`) |
| `unit_price_minor` | per-unit pence; for a listing of N items at £P total, `(P × 100) / N` rounded down |
| `currency` | `"GBP"` |
| `vat_inclusive` | typically `true` for UK retail; `null` if ambiguous |
| `quantity` | items bundled in the single listing, or 1 |
| `quantity_unit` | `"pack"`, `"lot"`, `"unit"` |
| `minimum_order_quantity` | `null` (eBay has no MOQ) |
| `price_tiers` | single-tier array — eBay typically lists one price |
| `ship_from_country` | from the "Item location" string on the listing (`GB` if in UK) |
| `includes_shipping` | `true` if postage is `£0.00 / Free`; else `false`; add postage to total if needed |
| `is_used_or_refurbished` | `false` (filter held) |
| `seller_identifier` | seller username |
| `match_confidence_bps` | rubric from Gem Imports, minus 500 bps retail bias, minus further 500 bps if seller is new/low-feedback |
| `evidence` | 1–3 sentences citing title, pack/lot size, and per-unit math |

### Step 3: Handle Zero-Result and Blocked Waves

- Zero results across all waves → one entry with
  `search_failure_mode: "no_results_all_waves"`, empty `quotes`.
- Soft block → `search_failure_mode: "source_soft_blocked"`.

### Step 4: Emit Result Contract

Write to `result.json` per the schema.

### Step 5: Close Browser

```
agent-browser close
```

---

## Canary
Search three representative goods. Verify every quote has the BIN + New
filter applied (no auction items, no used items). Verify `currency = GBP`
and `ship_from_country = GB` for the chosen quotes.

## Run Manifest Instructions
See Gem Imports SKILL.md. `agent_claimed_source_url` is
`https://www.ebay.co.uk/`. In Decisions Made, call out which listings
you rejected for auction / used / low seller feedback, and which you
preferred for multi-item bundles.
