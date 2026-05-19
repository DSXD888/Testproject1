---
skill_id: supplier-price-enrichment/amazon
skill_version: 2026.04.21-1
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
validator_overlay: validation/overlay-customs-classification
validator_overlay_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: local_state
promotion_stage: candidate
worker_behavior_version: supplier-price-enrichment/amazon@2.0.0
validator_behavior_version: validation/overlay-customs-classification@1.0.0+1.0
version_change_affects_behavior: true
gate_a_family_key: supplier-price-enrichment
gate_a_family_prequalified: false
gate_b_exact_slice_key: supplier-price-enrichment/amazon::amazon::supplier-price-enrichment/amazon@2.0.0::validation/overlay-customs-classification@1.0.0+1.0
gate_b_exact_slice_ready: false
maxPages: 3
maxScrollCycles: 2
browser_attach_mode: cdp_persistent_profile
browser_profile_path: $HOME/.local/share/govt-scraping/chrome-profiles/amazon
---

# supplier-price-enrichment/amazon

Searches Amazon UK (amazon.co.uk) for UK-market product pricing. Shares the structural contract with `supplier-price-enrichment/alibaba`; read both files when executing. This file specifies the Amazon L5 deltas: alias-family query waves, structured title vocabulary, concrete ASIN variant selection, browser-first operation after the PA-API 5.0 sunset, and validator-overlay evidence binding.

## Use when
Use for `candidate_type IN ('exact_sku','standard_commodity','brand_family','regulated_goods')` when the dispatcher routes to `source_id = "amazon"`. Amazon UK is a **UK-retail benchmark ceiling** — results land as `price_signal_role: "benchmark_ceiling"`. Prime / Amazon-fulfilled listings are preferred for benchmark purposes because their delivery semantics are well-defined.

## Do not use when
Do not use for bulk/wholesale-only candidates (`candidate_type = "bundle"` when the tender specifies pallet quantities) — Amazon's retail unit-of-sale is too small to be a meaningful benchmark. Don't use for custom-fabricated items or installed goods.

## Browser Attach Protocol (MANDATORY — more strict than other sources)

Same CDP + persistent-profile mandate as Alibaba. Amazon's anti-bot is the most aggressive of the four marketplaces this platform searches — **`--headed` is non-negotiable**, and the persistent profile MUST be kept warm with an active signed-in Amazon account. Cold/anonymous searches from a fresh Chrome are frequently interstitial-challenged ("Are you a robot?" CAPTCHA) even before the first result page renders.

- **Profile path:** `$HOME/.local/share/govt-scraping/chrome-profiles/amazon` (distinct from Alibaba / 1688 / eBay).
- **Landing URL:** `https://www.amazon.co.uk/`.
- **Search URL template:** `https://www.amazon.co.uk/s?k=<urlencoded_query>&language=en_GB&__mk_en_GB=%C3%85M%C3%85%C5%BD%C3%95%C3%91` (language + mk parameters pin English UK).
- **Authenticated header tell:** `"Hello, <first_name>"` greeting visible in the nav bar AND an `/gp/css/homepage.html` link that returns an account page (not a login redirect). Anonymous state is **NOT permitted** — emit `failure_code: SOURCE_AUTH_STATE_INVALID` and short-circuit per the Alibaba pattern if the session is unauthenticated. Amazon anti-bot treats un-logged-in sessions as higher-risk and may shadow-serve degraded results.
- **Expected locale:** `en-GB`, currency `GBP`. Hard-drift on `currency != "GBP"` or `locale != "en-GB"` — Amazon UK doesn't serve `English-CHF` even when accessed from a Swiss IP because the site is per-domain not per-IP.
- **CAPTCHA handling:** if the header or content region surfaces a "Type the characters you see" / "We just need to make sure you're not a robot" interstitial, emit `failure_code: SOURCE_CAPTCHA_BLOCKED` and route to operator session recovery. Do not solve CAPTCHA or type credentials.
- **PA-API prohibition:** PA-API 5.0 is sunset for this workflow as of 2026-04-30. Do not call PA-API, do not request PA-API credentials, and do not fall back to a stored PA-API result. If any run configuration requests PA-API, emit `AMAZON_PAAPI_SUNSET_DETECTED` and `AMAZON_BROWSER_FIRST_REQUIRED`.

## Inputs

Same `job_parameters` shape as Alibaba. The full SearchProfile batch from the dispatcher MUST be read end-to-end; do NOT collapse to head_noun. The hard-conflict filter relies on every structured slot.

Amazon-specific adjustments when walking query waves:

- **Alias-family waves:** Expand `head_noun` and `noun_aliases_en` into alias-family query waves before broadening. Example: `hex bolt` may run `hex bolt`, `hex screw`, and `hex cap screw` waves. Record `strategy: "alias_family"` for these waves and preserve the originating alias in evidence.
- **Structured title vocabulary:** Parse titles and detail-page headings into structured tokens: brand, model, pack size, material, dimensions, standard/compliance tokens, and variant attributes. Structured title tokens are used for hard-conflict filtering and for concrete ASIN selection.
- **Wave 2 (market_frame):** use `commercial`, `professional`, `trade` framing. Avoid adding `industrial` as a framing token to consumer-adjacent categories — Amazon routes `industrial` into its Industrial & Scientific storefront which has different ranking semantics and smaller inventory than the main marketplace.
- **Wave 4 (compliance_off) for `regulated_goods`:** Amazon often surfaces standards (`CE`, `UKCA`, `BS EN` codes) in the A+ content / product-details panel rather than the listing title, so the compliance-off retry is particularly valuable here.

## Per-candidate execution protocol

Same 6-step flow as Alibaba, with these Amazon-specific refinements:

1. **Fulfillment class matters.** Each Amazon listing has a fulfillment tell: `Amazon-fulfilled` / `Prime` / `Fulfilled by Merchant`. Record it in `supplier_quotes[].evidence.fulfillment_class` and treat as follows:
   - `Amazon-fulfilled` / `Prime`: `includes_shipping: 1`, `is_ready_to_ship: 1`, `lead_time_days: 1-2` for Prime UK.
   - `Fulfilled by Merchant`: record the merchant-stated delivery cost + ETA, leave `includes_shipping: 0`.
   - Used-marketplace listings (from Amazon Warehouse or third-party used sellers): emit with `is_used_or_refurbished: 1` and appropriate `condition`.

2. **Variant-ASIN normalization.** An Amazon search result row often represents a product family (e.g. "Bosch GSR 18V-50 — 4 variations"). Click through to the detail page ONLY for the top `max_detail_fetches` candidates; on the detail page, select the ASIN variant whose declared attributes best match the SearchProfile's required critical_attributes, and emit that specific ASIN's price. Do NOT emit the family-level "from £X" pseudo-price — downstream margin gates reject ambiguous quote_class.

   - Quote emission requires a concrete 10-character ASIN, typically `B0...`, from the selected variant.
   - If the detail page exposes a variant family but no variant matches the SearchProfile's required attributes, emit `AMAZON_AMBIGUOUS_VARIANT_FAMILY` and do not emit a supplier quote.
   - Never use a family-level `from GBP` pseudo-price as `unit_price_gbp_pence`.

3. **VAT inclusion.** Amazon UK B2C listings show VAT-inclusive prices by default; `vat_inclusive: 1`. For Business-account visible items with a "Business Price (ex VAT)" line item, emit the ex-VAT price with `vat_inclusive: 0` and record the B2C-equivalent in `evidence.retail_price_gbp_pence_vat_inc`.

4. **Sponsored vs organic.** Sponsored listings are explicitly marked `"Sponsored"`. Record `quote_class: "sponsored_ad"` for those and `quote_class: "organic_listing"` otherwise. Sponsored listings are still valid benchmark signals but downstream may weight them differently.

5. **Seller reputation & reviews.** For `regulated_goods`, require ≥ 4.0 average review rating AND ≥ 50 reviews before emitting as a usable quote. Below that threshold, emit to `warnings[]` as `LOW_CONFIDENCE_SELLER_COMPLIANCE` rather than `supplier_quotes[]`.

6. **Validator overlay binding.** Every emitted quote records enough evidence for `validation/overlay-customs-classification`: exact query text, alias family, ASIN, selected variant attributes, structured title tokens, fulfillment class, VAT state, review count, and transcript path. Missing evidence reduces confidence or suppresses the quote; it never silently promotes Amazon evidence to an executable wholesale quote.

## Outputs

Same `result.json` + `run-manifest.md` contract as Alibaba. Key invariants:

- `source_id: "amazon"`
- `source_name: "Amazon"` on every `supplier_quotes[]` entry
- `price_signal_role: "benchmark_ceiling"`
- `currency: "GBP"`
- `region: "UK"`

## Failure Codes

Same set as Alibaba, plus Amazon-specific:

- `AMAZON_PAAPI_SUNSET_DETECTED` — job parameters, environment, or run manifest attempted to use PA-API after the 2026-04-30 sunset.
- `AMAZON_BROWSER_FIRST_REQUIRED` — browser-first execution was required but not available, or PA-API fallback was requested.
- `AMAZON_AMBIGUOUS_VARIANT_FAMILY` — a listing/detail page represented a variant family and no concrete ASIN variant matched required attributes.
- `SOURCE_CAPTCHA_BLOCKED` — Amazon serves "Are you a robot?" interstitials more aggressively than other sources; handled by short-circuit not retry.
- `AMAZON_BUSINESS_LOGIN_REQUIRED` — some B2B listings require an Amazon Business account, not just consumer login. Degrade cleanly by emitting the consumer price + a warning.
- `VARIANT_ASIN_AMBIGUOUS` — legacy alias for `AMAZON_AMBIGUOUS_VARIANT_FAMILY`; normalize to the Amazon-specific L5 code in result metadata.

## Non-goals

- Do not use PA-API, PA-API credentials, paid data APIs, or stored PA-API responses.
- Do not authenticate, solve CAPTCHA, type credentials, or repair cookies.
- Do not emit executable wholesale quotes; Amazon evidence remains `benchmark_ceiling`.
- Do not emit family-level pseudo-prices or ambiguous variant prices.
- Do not persist directly to SQLite; emit schema-valid result and manifest artifacts only.

## Resume Strategy

Checkpoint by `profile_id` + wave. After every completed query wave, persist `last_completed_profile_id`, `last_completed_wave`, `completed_profile_ids`, and the browser attach state. On resume, skip fully completed profiles and continue the inflight profile at `last_completed_wave + 1`. The persistent Chrome profile is the durable browser state; do not import cookies.

## Pagination Bounds

`maxPages: 3` and `maxScrollCycles: 2` are hard limits per query wave. Canary runs are stricter: at most two query waves and one detail-page fetch per case.

## Canary

The L5 canary set in `canary.inputs.jsonl` covers fasteners, packaging cartons, examination gloves, ambiguous variant-family rejection, and browser-first/PA-API-sunset enforcement. Passing canaries must emit `price_signal_role: "benchmark_ceiling"` and a concrete ASIN variant; rejection canaries must emit the documented Amazon-specific failure code with no supplier quote.

## Run Manifest Instructions

Every run emits `run-manifest.md` with YAML frontmatter containing `manifest_version`, `skill_id`, `skill_version`, `job_id`, `session_id`, `execution_mode`, `started_at`, `completed_at`, and `outcome_class`.

The body must include `Summary`, `Steps Taken`, `Decisions Made`, `Warnings`, and `Evidence`. Evidence must cite each alias-family query, structured title vocabulary extraction, ASIN variant decision, PA-API prohibition check, listing URL, observed locale/currency, fulfillment class, review count for regulated goods, and agent-browser transcript path. Do not claim a quote, variant match, or browser-first compliance decision without an evidence anchor.
