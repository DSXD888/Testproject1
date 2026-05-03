---
skill_id: marketplace-search/amazon-uk
skill_version: 1.1.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
requires_cdp: true
cdp_default_port: 9222
maxPages: 10
maxScrollCycles: 3
---

# marketplace-search/amazon-uk

## Use when
Use this skill when the daemon needs a UK retail benchmark ceiling from
Amazon UK (`https://www.amazon.co.uk/`) for `product_candidates` whose
`search_eligibility_state` is `eligible` or `marginal`.

## Do not use when
Do not use for wholesale/trade pricing (Gem Imports, MX Wholesale), for
Chinese floor pricing (Alibaba), or for ingestion.

## Inputs
- `source` — always `"amazon-uk"`.
- `candidates[]` — see the **Gem Imports SKILL.md** for the full field map.
  The daemon sends you the complete structured context per candidate:
  identity (`product_candidate_id`, `search_profile_id`, `name`,
  `head_noun`), full `specification_text` prose, `query_waves[]`, and
  structured anchors (`standards[]`, `critical_attributes`, `numeric_specs`,
  `material`, `quantity`, `quantity_unit`) plus parent context
  (`lot_title`, `lot_description`, `notice_title`, `notice_summary`,
  `buyer_name`, `cpv_codes`).

### Using the full candidate context when scoring (non-negotiable)

Never score an Amazon listing on `name` + `head_noun` alone. Before
shortlisting, verify:
1. **`cpv_codes`** — Amazon's own category should be consistent with the
   tender's CPV (e.g. CPV `44221111-6` = Double-glazed units ⇒ look for
   construction/glazing categories on Amazon, not "decor" or "Windows &
   Doors > Decorative").
2. **`standards`** from the candidate — if it cites `BS EN 1279`, the
   Amazon listing either cites it too or scores much lower.
3. **`specification_text`** — pull size / grade / material / finish
   explicitly, verify they're in the listing's bullets / A+ content.

Record the cross-references in each quote's `evidence` field.

## Outputs
- `result.json` matching `result.schema.json`. Every candidate must
  appear in `search_results` at least once.

**Critical:** prices captured here are consumer retail. The daemon
applies a configurable wholesale discount (default 30%) downstream; the
skill MUST store the raw retail price unchanged.

## Non-goals
- Logging in, using 1-click purchase, applying coupons, adding to cart.
- Converting retail → estimated wholesale (daemon responsibility).

## Failure Codes
- SOURCE_CHANGED
- SOURCE_SOFT_BLOCKED
- SOURCE_CAPTCHA_BLOCKED (Amazon frequently shows "Are you a robot?")
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- AGENT_STALLED

## Pagination Bounds
- `maxPages`: 10
- `maxScrollCycles`: 3

---

## Playbook

You are a retail-benchmark price-search agent. Amazon is noisy — your
job is to find the **best per-unit price** for a product matching each
candidate, not just the first result.

### Mandatory CDP Discipline

Use the operator persistent CDP browser for this public source so the run stays visible on the shared browser surface and preserves locale/session state. Do not launch a fresh browser profile and do not close the operator browser.

Before navigation:

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, write a valid blocked `result.json` with `search_failure_mode: "source_soft_blocked"` for every remaining candidate and a warning/blocker code `CDP_CONNECT_FAILED`. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

### Step 0: Read Job Parameters

Parse `candidates` and their `query_waves`.

### Step 1: Open Amazon UK

```
agent-browser --cdp ${CDP_PORT} open "https://www.amazon.co.uk/"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

Dismiss cookie / consent banners. Do NOT log in.

If the page shows a captcha / "Are you a robot?" interstitial, refresh
once. If the captcha persists, set `search_failure_mode:
"source_captcha_blocked"` on every remaining candidate's wave entry
and terminate the session gracefully.

### Step 2: For Each Candidate, Walk Query Waves

For each candidate, iterate `query_waves`. Stop at the first wave that
produces ≥1 shortlisted quote with `match_confidence_bps >= 4000`.

#### 2a. Execute the search

Use Amazon's query-string search URL:

```
agent-browser --cdp ${CDP_PORT} open "https://www.amazon.co.uk/s?k=<URL-encoded query>"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

#### 2b. Shortlist up to 3 listings — **prefer bulk / multi-pack**

This is the Amazon-specific rule: when scanning results, **actively
prefer listings that offer the best per-unit price**. Signals:

- Multi-pack bundles ("Pack of 10", "Box of 50", "Case of 100") —
  divide the listing price by the pack quantity to get the per-unit
  price and compare across listings.
- Amazon Business / B2B listings when present (labeled "Business
  price" on the result card).
- "Subscribe & Save" tags — ignore the S&S percentage; use the regular
  one-time-purchase price, because S&S is not a procurement contract.
- "Amazon's Choice" and "Best Seller" badges are signals of
  category-fit, not price quality.

Reject: used / refurbished listings, renewed listings, listings whose
title clearly mismatches the candidate (e.g. "Toy metal railings" when
the candidate is real metal fencing).

#### 2c. Open each shortlisted detail page

```
agent-browser --cdp ${CDP_PORT} click "@<ref of product title link>"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

Scroll the page to reveal bulk/quantity options:

```
agent-browser --cdp ${CDP_PORT} scroll down 800
agent-browser --cdp ${CDP_PORT} snapshot
```

Check whether a "quantity discount" table is shown — some Amazon
listings expose tiered pricing at high quantities. If present, capture
every tier in `price_tiers`.

#### 2d. Extract raw fields

| Field | Notes |
|-------|-------|
| `product_name` | product title |
| `source_url` | current URL (strip tracking query params) |
| `source_listing_id` | ASIN (last path segment with 10 alphanumeric chars) |
| `unit_price_minor` | **per-unit** pence — for a pack of N priced at £P, this is `(P × 100) / N`, rounded down |
| `currency` | `"GBP"` |
| `vat_inclusive` | `true` (Amazon UK displays VAT-inclusive retail prices) |
| `quantity` | pack size visible in title / bullets, or 1 |
| `quantity_unit` | `"pack"`, `"case"`, `"box"`, `"unit"` |
| `minimum_order_quantity` | `null` (no MOQ on Amazon) |
| `price_tiers` | quantity-discount tiers if visible, else a single tier |
| `ship_from_country` | `"GB"` unless "Ships from" says otherwise |
| `includes_shipping` | `true` if "FREE UK Delivery" shown; else `false` |
| `is_used_or_refurbished` | `false` (reject used at shortlist time) |
| `match_confidence_bps` | see Gem Imports rubric |
| `evidence` | 1–3 sentences: title substring match + pack size + per-unit math |

#### 2e. Penalize retail bias in match confidence

Amazon retail is an **upper bound**, not a wholesale match. Subtract
500 bps from the rubric score for "retail benchmark bias". A perfect
title match still caps around `match_confidence_bps: 9500`.

### Step 3: Handle Zero-Result, Captcha, Blocked

- Zero results across all waves → one `search_results` entry with
  `search_failure_mode: "no_results_all_waves"` and empty `quotes`.
- Captcha on any page load → `search_failure_mode:
  "source_captcha_blocked"`, empty `quotes`, move on.

### Step 4: Emit Result Contract

Write to `result.json`. Example quote for an envelopes candidate:

```json
{
  "product_name": "C5 White Envelopes Box of 500 Self Seal",
  "source_url": "https://www.amazon.co.uk/dp/B07XXXX",
  "source_listing_id": "B07XXXXXXX",
  "unit_price_minor": 10,
  "currency": "GBP",
  "vat_inclusive": true,
  "quantity": 500,
  "quantity_unit": "box",
  "minimum_order_quantity": null,
  "price_tiers": [
    {"min_qty": 1, "unit_price_minor": 10, "currency": "GBP"}
  ],
  "ship_from_country": "GB",
  "includes_shipping": true,
  "is_used_or_refurbished": false,
  "match_confidence_bps": 8500,
  "evidence": "Title: 'C5 White Envelopes Box of 500'. £50/500 = 10p per unit. Matches C5 envelopes candidate."
}
```

### Step 5: Leave Browser Open

Do not close or reset the operator browser. Leave the last page visible for inspection.

---

## Canary
Search three representative goods (envelopes, sealed double-glazed
units, softwood timber). Verify `currency = GBP`, verify the skill
prefers multi-pack listings over single-unit when per-unit price is
lower, and verify a captcha interstitial is reported as
`source_captcha_blocked` rather than silently failing.

## Run Manifest Instructions
See Gem Imports SKILL.md. `agent_claimed_source_url` is
`https://www.amazon.co.uk/`. Note in Decisions Made any places where
you preferred a multi-pack listing for lower per-unit price — this is
the Amazon-specific judgment call that validator review cares about.
