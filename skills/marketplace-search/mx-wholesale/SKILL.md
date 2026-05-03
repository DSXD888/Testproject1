---
skill_id: marketplace-search/mx-wholesale
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

# marketplace-search/mx-wholesale

## Use when
Use this skill when the daemon needs UK wholesale/trade pricing from
MX Wholesale (`https://www.mxwholesale.co.uk/`) for a batch of
`product_candidates` whose `search_eligibility_state` is `eligible`
or `marginal`.

## Do not use when
Do not use for Chinese floor pricing, UK retail ceilings, or ingestion.

## Inputs
- `source` — always `"mx-wholesale"`.
- `candidates[]` — see the **Gem Imports SKILL.md** for the full field map.
  Beyond the basic identity fields (`product_candidate_id`,
  `search_profile_id`, `name`, `head_noun`, `specification_text`,
  `query_waves[]`), the daemon also sends structured anchors (`standards[]`,
  `critical_attributes`, `numeric_specs`, `material`, `quantity`,
  `quantity_unit`) plus parent context (`lot_title`, `lot_description`,
  `notice_title`, `notice_summary`, `buyer_name`, `cpv_codes`).

### Using the full candidate context when scoring (non-negotiable)

MX Wholesale's catalogue is narrow — wholesale household / cleaning /
catering / tobacco / small-electrical. Before shortlisting any listing:
1. Check **`cpv_codes`** — if the tender's CPV is a construction /
   glazing / engineering code, this source almost certainly doesn't
   stock it; be prepared to record `no_results_all_waves` quickly.
2. Read **`specification_text`** for the actual product spec (size /
   grade / pack-size / delivery-window) — match against listing bullets.
3. Check **`standards`** — if the candidate cites an industry standard,
   score much lower when the MX Wholesale listing doesn't carry a
   comparable spec tag.

Record these cross-references in each quote's `evidence` field.

## Outputs
- `result.json` matching `result.schema.json`, one entry per
  (candidate × wave executed), and every candidate represented at
  least once in `search_results`.

## Non-goals
- Logging in, opening a trade account, ordering, or any write action.
- Currency conversion; store raw pence, `currency: "GBP"`.

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

You are a UK wholesale price-search agent. Follow the same loop as the
Gem Imports skill; this playbook only highlights MX Wholesale specifics.

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

Identical to the Gem Imports playbook — parse `candidates` and their
`query_waves`.

### Step 1: Open MX Wholesale

```
agent-browser --cdp ${CDP_PORT} open "https://www.mxwholesale.co.uk/"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

MX Wholesale is a **trade supplier** that generally shows **ex-VAT
wholesale prices** to anonymous visitors. Some product lines require a
trade account to reveal pricing — those listings should be reported as
`search_failure_mode: "price_hidden_login_required"` rather than being
skipped silently.

Dismiss any cookie / consent banner before searching (click the accept
button). Do NOT create an account.

### Step 2: For Each Candidate, Walk Query Waves

For each candidate, iterate `query_waves` in ascending order. Stop at
the first wave that produces ≥1 shortlisted quote.

#### 2a. Execute the search

Use the site's own search URL when possible:

```
agent-browser --cdp ${CDP_PORT} open "https://www.mxwholesale.co.uk/search?q=<URL-encoded query>"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

If that URL shape fails, use the header search box:

```
agent-browser --cdp ${CDP_PORT} fill "input[name='q'], input[type='search']" "<query>"
agent-browser --cdp ${CDP_PORT} press Enter
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

#### 2b. Shortlist up to 3 listings

Use the same rubric as Gem Imports: head-noun alignment, spec overlap,
category sanity. Reject obvious non-matches.

MX Wholesale listings frequently ship in **fixed case / pack sizes**
(e.g. "Case of 144"). Capture pack size in `listing_pack_qty` context —
but for this skill store it in `quantity` and set `quantity_unit`
accordingly. The per-unit price you report in `unit_price_minor` must
match the **pack-adjusted unit price** (pack price ÷ pack qty, rounded
down to whole pence).

#### 2c. Extract raw fields

| Field | Notes |
|-------|-------|
| `product_name` | title |
| `source_url` | listing URL |
| `source_listing_id` | SKU / product code |
| `unit_price_minor` | per-unit pence (see pack note above) |
| `currency` | `"GBP"` |
| `vat_inclusive` | `false` when labeled "ex VAT"; `true` for "inc VAT"; `null` if ambiguous |
| `quantity` | pack size when sold-by-the-pack, else `1` |
| `quantity_unit` | `"pack"`, `"case"`, `"unit"` as shown |
| `minimum_order_quantity` | declared MOQ, else `null` |
| `price_tiers` | trade tiers if visible (frequent at MX Wholesale); else single-tier |
| `ship_from_country` | `"GB"` |
| `includes_shipping` | `false` unless banner says free delivery |
| `is_used_or_refurbished` | `false` |
| `match_confidence_bps` | see Gem Imports rubric |
| `evidence` | 1–3 sentences citing the title substring + spec signals |

#### 2d. Trade-price hidden

If a listing shows "Log in for price" / "Trade account required", do
NOT attempt to log in. Skip the listing, decrement `shortlisted_count`,
and continue. If every candidate result on the page hides its price,
emit the wave's `search_results` entry with
`search_failure_mode: "price_hidden_login_required"` and continue to the
next candidate.

### Step 3: Handle Zero-Result and Blocked Waves

Same rules as Gem Imports. On `no_results_all_waves`, still emit one
entry with empty `quotes: []` so the daemon can record the failed search
attempt. On soft blocks, emit `search_failure_mode: "source_soft_blocked"`.

### Step 4: Emit Result Contract

Write the result to `result.json` in the working directory following the
schema (see Gem Imports SKILL.md for the full shape).

### Step 5: Leave Browser Open

Do not close or reset the operator browser. Leave the last page visible for inspection.

---

## Canary
Search three representative goods (timber, double-glazed, envelopes).
Verify that at least some listings expose prices to anonymous visitors
(i.e. the site is not 100% gated). Verify `currency = GBP`. Treat any
persistent login wall as `price_hidden_login_required` without
attempting to authenticate.

## Run Manifest Instructions
See Gem Imports SKILL.md — same frontmatter fields and narrative
sections. `agent_claimed_source_url` is `https://www.mxwholesale.co.uk/`.
