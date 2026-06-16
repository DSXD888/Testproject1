---
skill_id: marketplace-search/gem-imports
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

# marketplace-search/gem-imports

## Use when
Use this skill when the daemon needs UK wholesale/trade pricing from Gem Imports
(`https://www.gemimports.co.uk/`) for a batch of `product_candidates` whose
`search_profiles.search_eligibility_state` is `eligible` or `marginal`.

## Do not use when
Do not use for Chinese floor pricing (use `marketplace-search/alibaba`), for
UK retail benchmark ceilings (use `marketplace-search/amazon-uk` or
`marketplace-search/ebay-uk`), or for procurement notice ingestion.

## Inputs
- `source` — always `"gem-imports"`.
- `candidates[]` — batch of candidates to search. Each candidate carries
  BOTH identity + search-plan fields AND a rich structured-context payload
  extracted upstream from the tender PDF + parent notice. You MUST read the
  full context, not just `head_noun`, before scoring any listing.
- Optional `max_candidates` upper bound if the daemon wants to cap session scope.

### Per-candidate field map

| Field | Type | What it contains / how to use it |
|-------|------|----------------------------------|
| `product_candidate_id` | string | Primary key — echo back verbatim in every `search_results` entry. |
| `search_profile_id` | string | Same — echo back verbatim. |
| `name` | string | Full candidate name, e.g. *"Double-glazed unit (sealed insulating glass unit, BS EN 1279-compliant, argon-filled, warm-edge spacer)"*. |
| `head_noun` | string | Normalised noun phrase for keyword search, e.g. `"double glazed unit"`. No hyphens. |
| `specification_text` | string | Full spec prose from the PDF extraction — contains size / grade / finish / warranty / delivery expectations. **Always read this before scoring.** |
| `query_waves` | array | Ordered wave plan (strategy + query) from Stage 8. Execute in wave-number order; stop at the first wave producing a shortlisted quote. |
| `standards` | array | **Structured compliance-standard list** (e.g. `["BS EN 1279"]`). The single strongest anchor for specialist goods — a listing that doesn't cite the same standard in its title/spec block scores much lower. |
| `critical_attributes` | object/null | Structured must-have attributes (may be null on current data). |
| `numeric_specs` | object/null | Structured dimensions / grades / weights (may be null on current data). |
| `material` | string/null | e.g. `"galvanised steel"`. Different material = hard-reject the listing. |
| `quantity`, `quantity_unit` | number/string | Declared order qty + unit. Informs MOQ-fit and tier-pricing preference. |
| `lot_title` | string/null | The parent lot's short title, e.g. *"Supply of Double Glazed Units"*. Tells you which lot-scope this candidate sits in when a notice has multiple lots. |
| `lot_description` | string/null | The parent lot's description — framework-specific context including delivery expectations, who's buying, which sub-scope. |
| `notice_title` | string | The procurement notice title — grounds the candidate in the overall tender. |
| `notice_summary` | string/null | Notice-level summary, typically from the OCDS release body. |
| `buyer_name` | string/null | The buying authority (e.g. *"Bristol City Council"*). |
| `cpv_codes` | string/null | EU-standardised Common Procurement Vocabulary codes, pipe-separated, e.g. *"14820000-5 - Glass \| 44221111-6 - Double-glazed units"*. **Strongest institutional category anchor** — cross-check against the listing's own categorisation on the marketplace. |

### Using the full candidate context when scoring (non-negotiable)

The rule: **never score a listing using `name` + `head_noun` alone**. Before
adding a listing to the shortlist, cross-reference AT LEAST:

1. `specification_text` (for size / grade / finish / warranty match signals), AND
2. `standards` if present (a DGU listing that doesn't cite BS EN 1279 is not the same product), AND
3. `cpv_codes` + `lot_title` (to validate that the listing is in the right product category at institutional granularity).

A listing that matches the head-noun but misses the `standards` / `cpv_codes`
context is *similar*, not *equivalent*, and should score ≤ 4 500 bps unless
the listing explicitly covers all named spec signals. Record the mismatch in
the quote's `evidence` field.

## Outputs
- `result.json` matching `result.schema.json` with one entry per
  (candidate × wave actually executed) in `search_results[]` and zero-or-more
  `quotes[]` per entry.
- Every candidate must appear at least once in `search_results`, even if no
  waves found matches (use `search_failure_mode: "no_results_all_waves"`).

## Non-goals
- Negotiating prices, placing orders, logging in as a trade account, creating
  accounts, or any write-side action. **`side_effect_class: none`.**
- Chinese or retail pricing (handled by sibling skills).
- Currency conversion — store raw pence, `currency: "GBP"`.

## Failure Codes
- SOURCE_CHANGED
- SOURCE_SOFT_BLOCKED
- SOURCE_CAPTCHA_BLOCKED
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- AGENT_STALLED

## Pagination Bounds
- `maxPages`: 10 per candidate (search result pages to browse)
- `maxScrollCycles`: 3 per page (infinite-scroll listings)

---

## Playbook

You are a UK wholesale price-search agent. Your job is to search Gem Imports
for supplier prices matching each `product_candidate` in the job parameters,
and return a result contract documenting every search attempt and every
usable quote.

### Mandatory CDP Discipline

Use the operator persistent CDP browser for this public source so the run stays visible on the shared browser surface and preserves locale/session state. Do not launch a fresh browser profile and do not close the operator browser.

Before navigation:

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# Ensure CDP is up. The daemon's pre-flight gate SHOULD have done this already,
# but defense in depth: bin/ensure-cdp-9222 is idempotent and fast (<1s no-op
# when CDP is already up; otherwise launches Chrome with the operator's
# persistent profile at ~/.chrome-automation and polls up to
# 15s). Returns non-zero only if launch genuinely failed. Absolute path because
# the skill subprocess cwd is the per-skill workdir, not the project root.
~/Desktop/testproject1/bin/ensure-cdp-9222

# Defensive reachability check. If this fails (helper exited non-zero, or a
# rare post-launch race), follow the "If CDP is unreachable" guidance below:
# write the skill-specific blocked result.json and stop.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, write a valid blocked `result.json` with `search_failure_mode: "source_soft_blocked"` for every remaining candidate and a warning/blocker code `CDP_CONNECT_FAILED`. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

### Step 0: Read Job Parameters

The daemon injects JSON into the prompt. Parse the top-level object and
extract the `candidates` array. Each candidate has:

- `product_candidate_id` (string) — primary key, echo back verbatim.
- `search_profile_id` (string) — primary key, echo back verbatim.
- `name` (string) — full candidate name for human matching.
- `head_noun` (string) — short noun phrase (e.g., "double-glazed").
- `specification_text` (string) — fuller description. Use for match judgment.
- `query_waves` (array) — ordered wave plan, each item has `wave` (int),
  `strategy` (string), and `query` (string).

If `candidates` is empty or missing, write a minimal valid result and exit:
```json
{"source":"gem-imports","cursor":"{}","continuation_required":false,"search_results":[]}
```

### Step 1: Open Gem Imports

```
agent-browser --cdp ${CDP_PORT} open "https://www.gemimports.co.uk/"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

Observe the landing page. Find the search input (usually a header search
bar) and the site's pricing convention. Gem Imports typically shows
**ex-VAT trade prices** when logged out — note this for every quote you
extract.

If the page shows a captcha, an age gate, a maintenance banner, or a
consent dialog: resolve it by clicking the accept/dismiss control, or if
blocked, set `search_failure_mode` to `"source_soft_blocked"` for the
current wave and continue to the next candidate.

### Step 2: For Each Candidate, Run Query Waves Until a Quote is Found

Iterate `candidates` in order. For each candidate `C`, walk its
`query_waves` in ascending `wave` order and stop as soon as you produce
at least one shortlisted quote (or all waves are exhausted).

For each wave `W`:

#### 2a. Execute the search

```
agent-browser --cdp ${CDP_PORT} open "https://www.gemimports.co.uk/search?q=<URL-encoded W.query>"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

If that URL shape does not work (404 / no results on a wave-1 query you
expect to match), fall back to the header search bar:

```
agent-browser --cdp ${CDP_PORT} fill "input[name='q'], input[type='search']" "<W.query>"
agent-browser --cdp ${CDP_PORT} press Enter
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

Record the `result_count` (visible total) and the `query_used` exactly
as submitted.

#### 2b. Shortlist up to 3 plausible matches

Read the result listings from the snapshot. For each result, judge
whether it plausibly matches the candidate using:

- **Head-noun alignment**: does the listing title contain `C.head_noun`
  or a close synonym?
- **Spec overlap**: does the listing mention any of the standards,
  dimensions, materials, or grades in `C.specification_text`?
- **Product-type sanity**: reject clearly wrong categories (e.g. a
  cleaning product when searching for timber).
- **Condition**: reject used/refurbished outright; `is_used_or_refurbished`
  should be `false` for every quote you record.

Pick at most 3 candidates to fully quote. Count everything visible under
`result_count`; count items you rejected as clearly wrong under
`hard_rejected_count`; count kept items under `shortlisted_count`.

#### 2c. Open each shortlisted detail page and extract

For each shortlisted listing, navigate to its detail page:

```
agent-browser --cdp ${CDP_PORT} click "@<ref of listing link>"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot
```

Extract the following **raw** fields (do NOT apply any wholesale discount,
do NOT convert currency — these are handled downstream):

| Field | Where to look |
|-------|---------------|
| `product_name` | `<h1>` / product title |
| `source_url` | current URL |
| `source_listing_id` | SKU / product code on the page (best-effort) |
| `unit_price_minor` | price × 100 (pounds → pence, integer) |
| `currency` | `"GBP"` |
| `vat_inclusive` | `false` if labeled "ex VAT" or "excl. VAT"; `true` if labeled "inc VAT"; `null` if unclear |
| `quantity` | usually `1` unless the listing is sold in a fixed pack |
| `quantity_unit` | `"unit"`, `"each"`, `"pack"`, `"m"`, `"m2"` as shown |
| `minimum_order_quantity` | MOQ if declared; else `null` |
| `price_tiers` | trade tiers if a bulk pricing table is shown; else single-tier array with the listed price |
| `ship_from_country` | `"GB"` |
| `includes_shipping` | `false` unless explicitly free-shipping |
| `lead_time_days` | if stated; else `null` |
| `is_used_or_refurbished` | `false` (reject used at shortlist time) |
| `match_confidence_bps` | 0–10000; see rubric below |
| `evidence` | 1–3 sentences: exact title substring matched + spec signals |

If the detail page fails to load or is missing a price, skip that listing
(decrement `shortlisted_count` by 1) and try the next one.

#### 2d. Score `match_confidence_bps`

Use this rubric:

- **Head-noun exact match** in title: +3000
- **Head-noun synonym** match: +1500
- **Standard / spec keyword** from `specification_text` appears in title
  or spec block: +2000
- **Material / grade** explicit match: +1500
- **Size / dimension** present and plausible vs candidate: +1000
- **Product-type sanity** (same category, e.g. glazing vs metalwork): +1000
- Subtract 2000 if the listing is a generic bundle / accessory rather
  than the head noun itself.

Clamp to `[0, 10000]`. A quote with score < 4000 should be dropped
(and `shortlisted_count` decremented).

### Step 3: Handle Zero-Result and Blocked Waves

If a wave returns zero hits AND you still have later waves: continue.

If **all** waves for a candidate return zero hits: emit a single
`search_results` entry for the final wave attempted with
`search_failure_mode: "no_results_all_waves"` and empty `quotes: []`.

If a wave is blocked (captcha / 403 / geo-block), emit an entry with
`search_failure_mode: "source_soft_blocked"` and empty `quotes: []`;
then move on to the next candidate.

### Step 4: Emit the Result Contract

Build the final JSON matching `result.schema.json` and **write it to
`result.json` in the current working directory**:

```json
{
  "source": "gem-imports",
  "cursor": "{}",
  "continuation_required": false,
  "search_results": [
    {
      "product_candidate_id": "pc-...",
      "search_profile_id": "sp-...",
      "query_used": "double glazed units 4mm toughened",
      "wave_number": 1,
      "is_recovery_wave": false,
      "result_count": 24,
      "hard_rejected_count": 15,
      "shortlisted_count": 2,
      "locale": "en-GB",
      "search_failure_mode": null,
      "quotes": [
        {
          "product_name": "4mm Toughened Double Glazed Unit 600×900mm",
          "source_url": "https://www.gemimports.co.uk/...",
          "source_listing_id": "GEM-12345",
          "unit_price_minor": 2500,
          "currency": "GBP",
          "vat_inclusive": false,
          "quantity": 1,
          "quantity_unit": "unit",
          "minimum_order_quantity": null,
          "price_tiers": [
            {"min_qty": 1, "unit_price_minor": 2500, "currency": "GBP"}
          ],
          "ship_from_country": "GB",
          "includes_shipping": false,
          "is_used_or_refurbished": false,
          "match_confidence_bps": 7500,
          "evidence": "Title match: '4mm Toughened Double Glazed Unit'. Size 600×900mm matches DGU spec."
        }
      ]
    }
  ]
}
```

### Step 5: Leave Browser Open

Do not close or reset the operator browser. Leave the last page visible for inspection.

---

## Canary
Search three representative physical-goods candidates (timber, sealed
double-glazed units, metal fencing), verify page shape, verify
`currency = GBP`, verify prices are rendered as ex-VAT trade prices
(Gem Imports default), and treat HTTP `403` / captcha interstitials as
`SOURCE_SOFT_BLOCKED` rather than failing the run.

## Run Manifest Instructions

Produce a run manifest the daemon will finalize into `run-manifest.md`.

Required YAML frontmatter fields:
- manifest_version
- skill_id
- skill_version
- job_id
- session_id
- execution_mode
- started_at
- completed_at
- outcome_class
- items_processed (= `candidates.length`)
- items_skipped (= candidates with `no_results_all_waves`)
- items_failed (= candidates where every wave errored)
- warnings_count
- agent_claimed_pages_visited
- agent_claimed_source_url (`https://www.gemimports.co.uk/`)
- agent_claimed_fingerprint_state (`wholesale_trade_site`)
- agent_claimed_locale_state (`en-GB`)
- agent_claimed_currency_state (`GBP`)

Required narrative sections:
- Summary
- Steps Taken
- Decisions Made (why you shortlisted / rejected particular listings)
- Warnings
- Evidence (link each quote back to a real product URL)

Major claims MUST anchor to observable evidence (URL + snippet from the
page, or the result-contract field the claim produced).
