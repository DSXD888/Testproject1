---
skill_id: marketplace-search/alibaba
skill_version: 1.3.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
maxPages: 10
maxScrollCycles: 3
requires_cdp: true
cdp_default_port: 9222
---

# marketplace-search/alibaba

## Use when
Use this skill when the daemon needs a **Chinese supplier floor price**
from Alibaba (`https://www.alibaba.com/`) for `product_candidates` whose
`search_eligibility_state` is `eligible` or `marginal`.

## Do not use when
Do not use for UK wholesale/trade, UK retail benchmark, or ingestion.

## MANDATORY execution mode — CDP into operator's logged-in Chrome

This skill **does not** spawn its own browser. It connects via CDP
(Chrome DevTools Protocol) to a Chrome instance the operator has
pre-staged with an Alibaba account logged in. This is the only way to
get past Alibaba's anti-bot interstitials and read tier pricing.

### Operator prerequisite (one-time setup)

Chrome must be running on port 9222 with a non-default user-data-dir.
The canonical launch:

```bash
# 1. Copy the operator's existing Chrome profile to a non-default path
#    (required by Chrome 136+ security policy: remote-debugging-port
#    is silently dropped when user-data-dir is the default one).
rsync -a \
  --exclude Cache --exclude "Code Cache" --exclude GPUCache \
  --exclude DawnGraphiteCache --exclude DawnWebGPUCache \
  --exclude GrShaderCache --exclude "Media Cache" --exclude "Download Cache" \
  --exclude "Service Worker/CacheStorage" \
  --exclude optimization_guide_model_store \
  "$HOME/Library/Application Support/Google/Chrome/Profile 10/" \
  "$HOME/.chrome-automation/Default/"
cp "$HOME/Library/Application Support/Google/Chrome/Local State" \
   "$HOME/.chrome-automation/Local State"

# 2. Launch Chrome pointed at the automation profile with debug port
open -a "Google Chrome" --args \
  "--remote-debugging-port=9222" \
  "--remote-allow-origins=*" \
  "--user-data-dir=$HOME/.chrome-automation" \
  "--profile-directory=Default" \
  "--no-first-run" "--no-default-browser-check"

# 3. Verify the Alibaba login cookie was copied + still valid by
#    opening https://www.alibaba.com/ and checking the header shows
#    the user avatar / account area instead of a bare "Sign in" link.
#    If the copied cookies weren't usable (v20 App-Bound encryption
#    drift, session expiry), log in once in the automation Chrome —
#    the cookies persist for subsequent skill runs.
```

The skill assumes port **9222** by default; override via the
`AGENT_BROWSER_CDP_PORT` env var if the operator chose a different port.

### What the agent MUST NOT do

- Launch `agent-browser` without `--cdp <port>` / `--auto-connect`.
  That spawns a fresh `Chrome for Testing` with no cookies → Alibaba
  will CAPTCHA-block every page.
- Attempt to solve a CAPTCHA programmatically. If CDP is set up
  correctly, you should not see one. If you do, emit
  `CDP_CONNECT_FAILED` or `SOURCE_CAPTCHA_BLOCKED` and terminate —
  this is a setup bug for the operator to fix, not a skill
  workaround.
- Log in / log out / change account settings / click "Contact
  Supplier" / anything that writes server-side state.

## Inputs
- `source` — always `"alibaba"` (skill emits this; the daemon ignores
  it and uses its own `source_id = "alibaba-search"`).
- `candidates[]` — batch of candidates; **see the Gem Imports SKILL.md for
  the full field map**. The daemon sends the complete structured context
  per candidate: identity (`product_candidate_id`, `search_profile_id`,
  `name`, `head_noun`), full `specification_text` prose, `query_waves[]`,
  and the richer anchors you NEED for disambiguation:
  `standards[]` (e.g. `["BS EN 1279"]`), `critical_attributes`,
  `numeric_specs`, `material`, `quantity` + `quantity_unit`, the parent
  lot (`lot_title`, `lot_description`), and the parent notice
  (`notice_title`, `notice_summary`, `buyer_name`, `cpv_codes`).
- Optional job_parameters fields:
  - `cdp_port` (integer) — override the default 9222.
  - `max_candidates` — upper bound on batch size.

### Using the full candidate context (non-negotiable on Alibaba)

Alibaba listings are wildly varied — the same head-noun ("double glazed
unit") surfaces single-piece DIY panels, 10 m² commercial IGU batches,
automotive glazing, smart-home tints, and display units. You MUST
disambiguate using structured context before scoring:

1. **`cpv_codes`** first — e.g. `44221111-6 - Double-glazed units` maps to
   the Architectural-glazing category. A listing for *"Double-Glazed Display
   Case"* (show-cabinet glass) is a different CPV-equivalent category; reject.
2. **`standards`** next — for this candidate, BS EN 1279 is the EU IGU
   sealing standard. Alibaba listings that mention CE / EN 1279 / ISO 12543
   score higher; generic listings without compliance citations score lower.
3. **`specification_text`** for dimension & feature match — e.g. *"max 28 mm
   thickness"*, *"argon-filled"*, *"warm-edge spacer"*, *"5-year guarantee"*.
4. **`lot_title` + `lot_description`** to understand the *context* — the
   buyer's lot is *"Supply of Double Glazed Units"* for *"Bristol response
   repairs and Wellington Road joiners shop"*, meaning the supplier must
   be able to ship small-batch replacement panels at UK-compatible sizes,
   not 10 000 m²-lot commercial orders.

Record this reasoning verbatim in each quote's `evidence` field so a
human reviewer can re-verify the match.

## Outputs
- `result.json` matching `result.schema.json`. Every candidate must
  appear in `search_results` at least once.
- **CRITICAL:** each quote MUST carry a `price_tiers` array with ≥ 1
  entry. Alibaba pricing is tiered; capturing the full ladder is the
  whole point.

## Non-goals
- Logging in, messaging suppliers, sending inquiries, clicking "Chat
  Now", any write-side action.
- Currency conversion. Alibaba's price display is **locale-bound to
  the operator's Chrome profile** — if the profile's "Deliver to"
  is `GB` / `English-GBP`, prices are shown in GBP; a US profile
  would show USD. Emit whichever currency code you observe; the
  daemon's `convert_to_gbp_pence` handles the conversion.

## Failure Codes
- `CDP_CONNECT_FAILED` — port not reachable, or
  `/json/version` returned a non-Chrome response. Operator setup bug.
- `SOURCE_CHANGED` — DOM structure no longer matches the Step 2c
  extractors (e.g. Alibaba changed class names again).
- `SOURCE_SOFT_BLOCKED` — intermittent 5xx / network flake.
- `SOURCE_CAPTCHA_BLOCKED` — shouldn't happen with a good CDP
  setup, but if it does, treat as "CDP session is not authenticated
  any more" and terminate so the operator can re-log.
- `PAGE_SHAPE_DRIFT` — search results or detail page missing
  expected containers.
- `LOCALE_OR_CURRENCY_DRIFT` — unexpected currency symbol (not £,
  $, €) that `convert_to_gbp_pence` can't handle.
- `AGENT_STALLED` — catch-all for timeouts.

## Pagination Bounds
- `maxPages`: 10
- `maxScrollCycles`: 3

---

## Playbook

You are a Chinese-supplier-floor price-search agent. You operate by
**attaching to an already-running Chrome** via Chrome DevTools
Protocol. You never spawn a browser yourself.

### Step 0: Read Job Parameters

Parse `candidates` and `query_waves`. Decide the CDP port:

```
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"
```

### Step 1: Verify CDP is reachable, then connect

Before doing any navigation:

```bash
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version" \
  || echo "CDP_UNREACHABLE"
```

If the response is empty / `CDP_UNREACHABLE` / not JSON containing
`"Browser": "Chrome/..."`: emit one `search_results` entry per
candidate with `search_failure_mode: "CDP_CONNECT_FAILED"`,
`result_count: 0`, empty `quotes: []`, and exit cleanly. Do NOT try
to launch a browser yourself.

Once verified, every subsequent agent-browser invocation MUST use the
CDP flag:

```
agent-browser --cdp ${CDP_PORT} <subcommand> ...
```

Quick sanity check after first navigate:

```
agent-browser --cdp ${CDP_PORT} open "https://www.alibaba.com/"
agent-browser --cdp ${CDP_PORT} wait 2000
agent-browser --cdp ${CDP_PORT} get title
```

If the title does NOT contain `Alibaba.com` (e.g. you got a CAPTCHA
or a `chrome-error://` URL), emit
`search_failure_mode: "SOURCE_CAPTCHA_BLOCKED"` and terminate.

### Step 2: For Each Candidate, Walk Query Waves

For each candidate, iterate `query_waves` in ascending `wave` order.
Stop at the first wave producing ≥ 1 shortlisted quote with
`match_confidence_bps ≥ 4000`.

#### 2a. Execute the search

Alibaba's canonical search URL:

```
agent-browser --cdp ${CDP_PORT} open \
  "https://www.alibaba.com/trade/search?SearchText=<URL-encoded query>"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

Read `result_count` from the result-count label (approximate — the
header typically reads "1,824 products found"). Record `query_used`
exactly as submitted.

#### 2b. Shortlist up to 2 listings

Pick at most **2** shortlisted candidates — deliberately lower than the
other marketplace skills (which allow 3). Fewer detail-page opens per
candidate is the single strongest lever we have against Alibaba's
per-session anti-bot rate-limiter: empirically, on the 2026-04-21 CDP
test, we got 2 good quotes from candidate 1 on wave 1 before the
bucket depleted; by candidate 4 the session was CAPTCHA-flagged. Two
shortlisted × ~5 candidates × inter-candidate waits keeps us below
the threshold most of the time.

Prefer listings with any of these badges visible in the card:
- "Verified Supplier" / "Verified Pro"
- "Trade Assurance"
- "Ready to Ship"
- High supplier transaction count (shown as "N transactions" or
  "N years in business")

Reject:
- "Single-piece customization" listings
- Trading-company listings when a manufacturer alternative is
  available for the same SKU family
- Off-category listings

#### 2c. Open each shortlisted detail page

```
agent-browser --cdp ${CDP_PORT} click "@<ref of title link>"
agent-browser --cdp ${CDP_PORT} wait 3000
```

Detail page is tall; scroll to reveal the tier-pricing block:

```
agent-browser --cdp ${CDP_PORT} scroll down 600
agent-browser --cdp ${CDP_PORT} wait 1000
```

Extract tiered pricing via these selectors (verified live on Alibaba
as of 2026-04-21):

```js
// Container
document.querySelector('[class*="module_price"]')

// Each tier row
document.querySelectorAll('[class*="price-item"]')
// Each row's text shape:
//   "100 - 999 square meters\n£112.02"
//   "1,000 - 9,999 square meters\n£74.68"
//   ">= 10,000 square meters\n£37.34"
```

Parse each row to `{min_qty, unit_price_minor, currency}`:
- `min_qty` = the first integer in the qty string (strip commas).
- Detect currency from the leading symbol: `£` → GBP, `$` or `US$`
  → USD, `€` → EUR.
- `unit_price_minor` = the price as an integer in MINOR units of the
  detected currency (pence for GBP, cents for USD / EUR). Multiply
  by 100 and round.

Set the top-level quote's `unit_price_minor` to the **lowest tier
price** (largest-qty tier) — the optimistic Chinese floor downstream
margin math expects.

#### 2d. Extract raw fields per quote

| Field | Notes |
|-------|-------|
| `product_name` | `<h1>` product title |
| `source_url` | current URL (keep the `product-detail` segment) |
| `source_listing_id` | numeric id in the URL (e.g. `1600440324243.html` → `1600440324243`) |
| `unit_price_minor` | lowest tier price, minor units of `currency` |
| `currency` | detected from price symbol (GBP / USD / EUR) |
| `vat_inclusive` | `null` (Alibaba prices are pre-VAT for the destination) |
| `quantity` | `min_qty` of the lowest tier |
| `quantity_unit` | `"piece"`, `"square meter"`, `"set"`, `"kg"`, etc., as shown on the tier row (text before the price) |
| `minimum_order_quantity` | supplier-declared MOQ (often ≥ quantity) |
| `price_tiers` | every parsed tier `{min_qty, unit_price_minor, currency}` |
| `ship_from_country` | country shown under supplier info (typically `"CN"`) |
| `includes_shipping` | `false` |
| `incoterm` | `"FOB"`, `"EXW"`, `"CIF"` if declared; else `null` |
| `is_ready_to_ship` | `true` if the Ready-to-Ship badge is visible |
| `seller_identifier` | supplier company name |
| `lead_time_days` | declared lead time, or `null` |
| `match_confidence_bps` | rubric from the Gem Imports skill (no retail-bias penalty) |
| `evidence` | 1–3 sentences: title, MOQ, tier summary, supplier verification status |

#### 2e. **MANDATORY** inter-candidate cool-down

Before moving to the next candidate in the batch, insert a 30-second
pause. This is the second of our two rate-limit mitigations (along with
the shortlist-of-2 cap in Step 2b). Without it, Alibaba's
per-session anti-bot bucket depletes after ~2–3 candidates and the
remaining candidates all land on CAPTCHA.

```
agent-browser --cdp ${CDP_PORT} wait 30000
```

Insert this call **at the end of each candidate's iteration**, AFTER
you have fully extracted Step 2c/2d for every shortlisted quote and
BEFORE you start Step 2a for the next candidate. Do NOT skip this on
the last candidate in the batch — other concurrent runs may be using
the same Chrome session.

Do NOT reduce the 30 000 ms value without operator authorisation —
it was calibrated empirically on 2026-04-21 after a CDP run that
captured 2 quotes then got CAPTCHA'd on candidate 4.

### Step 3: Handle Zero-Result / Blocked

- Zero results across all waves → one entry with
  `search_failure_mode: "no_results_all_waves"`, empty `quotes`.
- CAPTCHA mid-session → treat as CDP session expired:
  `search_failure_mode: "SOURCE_CAPTCHA_BLOCKED"`, empty `quotes`,
  stop processing remaining candidates (session is poisoned).
- DOM shape missing expected selectors →
  `search_failure_mode: "PAGE_SHAPE_DRIFT"`.

### Step 4: Emit Result Contract

Example quote (GBP example matching what the UK-locale CDP Chrome
returned on 2026-04-21):

```json
{
  "product_name": "Double Triple Glazed Aluminum Framed Tempered Insulating Glass Unit",
  "source_url": "https://www.alibaba.com/product-detail/Double-Triple-Glazed-Aluminum-Framed-Tempered_1600440324243.html",
  "source_listing_id": "1600440324243",
  "unit_price_minor": 3734,
  "currency": "GBP",
  "vat_inclusive": null,
  "quantity": 10000,
  "quantity_unit": "square meter",
  "minimum_order_quantity": 100,
  "price_tiers": [
    {"min_qty": 100,   "unit_price_minor": 11202, "currency": "GBP"},
    {"min_qty": 1000,  "unit_price_minor": 7468,  "currency": "GBP"},
    {"min_qty": 10000, "unit_price_minor": 3734,  "currency": "GBP"}
  ],
  "ship_from_country": "CN",
  "includes_shipping": false,
  "incoterm": null,
  "is_ready_to_ship": false,
  "seller_identifier": "<supplier company name from page>",
  "match_confidence_bps": 6500,
  "evidence": "Title match 'Double Triple Glazed … Insulating Glass Unit'. 3-tier pricing 100/£112.02 → 1000/£74.68 → 10000/£37.34 per m². Supplier type <trading/manufacturer>."
}
```

Write the final JSON to `result.json` in the working directory.

### Step 5: Leave Browser Open

Because you did not launch Chrome, you MUST NOT close it. The
operator's Chrome is persistent and shared across runs. Leave the last
navigated tab where it is — the daemon's next run will navigate away.

If you want to be tidy, close only the specific tab you opened
(keep `about:blank` / existing tabs alive):

```
agent-browser --cdp ${CDP_PORT} eval \
  'window.close()'   // only works for tabs you opened programmatically
```

(This is optional. Skipping it is safe.)

---

## Canary

Search three representative goods (double-glazed unit, metal
railings, envelopes) via `--cdp ${PORT}`. Verify:

- CDP reachable at `/json/version`.
- Navigation to `www.alibaba.com/trade/search?SearchText=...`
  succeeds with a title containing `Alibaba.com` (not a CAPTCHA page
  and not `chrome-error://`).
- At least one candidate yields ≥ 1 detail page with a readable
  `[class*="module_price"]` container and ≥ 2 `[class*="price-item"]`
  rows.
- Every quote has `currency` ∈ {GBP, USD, EUR}, `ship_from_country`
  populated, `price_tiers` length ≥ 1, and `unit_price_minor` equal
  to the lowest-tier (largest-qty) price.
- The operator's Chrome stays open and survives the run.

## Run Manifest Instructions

Produce a run manifest per the same frontmatter + narrative sections
used by the other marketplace-search skills:

- `agent_claimed_source_url` is `https://www.alibaba.com/`
- `agent_claimed_fingerprint_state` is `"cdp_attached_logged_in"`
  when the session verified auth on `/json/version`, or
  `"cdp_unreachable"` / `"cdp_attached_anonymous"` as applicable.
- `agent_claimed_locale_state` and `agent_claimed_currency_state`
  reflect whatever Alibaba served (driven by the operator's Chrome
  profile locale, e.g. `en-GB` + `GBP`).
- In Decisions Made, explicitly record the CDP connection step and
  the detected currency.
- In Evidence, quote at least one tier-pricing block verbatim so a
  reviewer can re-verify the `unit_price_minor` parse against the
  raw page text.

Major claims MUST anchor to observable evidence (URL + price-item
raw text, or the result-contract field the claim produced).
