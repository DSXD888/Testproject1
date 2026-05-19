---
skill_id: supplier-price-enrichment/ebay
skill_version: 2026.04.21-1
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
validator_overlay_version: 2026.04.21-1
validator_overlay: validation/overlay-customs-classification
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: local_state
promotion_stage: stable
worker_behavior_version: skills/supplier-price-enrichment/ebay@1.0.0
validator_behavior_version: validation/overlay-customs-classification@1.0.0+1.0
version_change_affects_behavior: true
gate_a_family_key: supplier-price-enrichment
gate_a_family_prequalified: true
gate_b_exact_slice_key: supplier-price-enrichment/ebay::ebay::skills/supplier-price-enrichment/ebay@1.0.0::validation/overlay-customs-classification@1.0.0+1.0
gate_b_exact_slice_ready: true
maxPages: 3
maxScrollCycles: 2
browser_attach_mode: cdp_persistent_profile
browser_profile_path: $HOME/.local/share/govt-scraping/chrome-profiles/ebay
---

# supplier-price-enrichment/ebay

Searches eBay UK (ebay.co.uk) for UK-market product pricing. Shares the structural contract with `supplier-price-enrichment/alibaba` — read both files when executing; this file specifies only what differs.

## Use when
Use for `candidate_type IN ('exact_sku','standard_commodity','brand_family','regulated_goods')` when the dispatcher routes to `source_id = "ebay"`. eBay UK is a **UK-retail benchmark ceiling** — results land as `price_signal_role: "benchmark_ceiling"` (NOT `optimistic_floor` like Alibaba/1688), and used/refurbished listings are explicitly admitted but flagged.

## Do not use when
Do not use for China-origin-only commodities that are not listed in the UK (those go to Alibaba/1688), or for `candidate_type = "custom_or_fabricated"` (bespoke manufacturing is not eBay's lane). Do not use for DPS framework-membership searches — we filter DPS at qualify.

## Browser Attach Protocol (MANDATORY)

Same CDP + persistent-profile mandate as `supplier-price-enrichment/alibaba`. Every forbidden-flag rule (`--profile Default`, `--auto-connect`, `--cdp <port>`, anti-detection flags, cookie import) applies. Single `batch` execution; `--headed` mandatory — eBay's homepage is usually permissive to anonymous browsers but **sponsored-listing and my-ebay features require login**, and headless Chrome is rate-limited more aggressively than headed.

- **Profile path:** `$HOME/.local/share/govt-scraping/chrome-profiles/ebay` (distinct from Alibaba / 1688).
- **Landing URL:** `https://www.ebay.co.uk/`.
- **Search URL template:** `https://www.ebay.co.uk/sch/i.html?_nkw=<urlencoded_query>&LH_PrefLoc=1` (`LH_PrefLoc=1` restricts to UK-based sellers — important for landed-cost semantics).
- **Authenticated header tell:** presence of "Hi, <username>" greeting and a non-empty My eBay dropdown. Anonymous state is **PERMITTED** for this skill — unlike Alibaba, eBay serves full search results to anonymous users, and many UK procurement-relevant categories don't require login. Emit a `warnings[]` entry noting anonymous mode but do NOT short-circuit on `SOURCE_AUTH_STATE_INVALID`.
- **Expected locale:** `en-GB`. Currency is **GBP** natively — no FX conversion needed for `unit_price_gbp_pence` when the listing denominates in GBP. For rare multi-currency listings (EUR / USD), convert using the daemon-supplied FX rate and record the original in `unit_price_original_minor` + `currency`.

## Inputs

Same `job_parameters` shape as Alibaba. You receive the full SearchProfile batch from the dispatcher — head_noun, candidate_type, match_archetype, critical_attributes_json, numeric_specs_json, standards_json, noun_aliases_en, preferred_terms, excluded_terms, query_waves_json, recall_risk_score_bps — and MUST read every field. Do NOT collapse to `head_noun`; the hard-conflict filter depends on the structured slots.

eBay-specific defaults when the profile's `query_waves_json` fires off wave 2 (market_frame):

- Use UK-market framing tokens: `commercial`, `professional`, `trade`, `wholesale`, `bulk`. Avoid `industrial` as the sole framing token for consumer-adjacent categories — eBay's category router treats it as a filter signal that excludes too much.

## Per-candidate execution protocol

Same 6-step flow as Alibaba, with these eBay-specific refinements:

1. **Listing-class separation.** Each eBay search result row carries a `condition` tell in the snapshot (`"New"`, `"Used"`, `"Refurbished"`, `"For parts or not working"`). Record it verbatim in `supplier_quotes[].is_used_or_refurbished` (boolean) and `listing_evidence.condition`. Required-criticality `condition: new` in `critical_attributes_json` hard-drops non-new listings.

2. **Buy-it-Now vs auction.** Only `"Buy It Now"` listings produce usable quotes. Auction listings with current bid + unresolved end-time are informational only — snapshot them but do NOT emit as `supplier_quotes`; record them in `warnings[]` as `AUCTION_LISTING_SKIPPED`.

3. **Seller-reputation gate.** Record the seller's feedback score + percentage on every emitted quote (`evidence.seller_feedback_score`, `evidence.seller_feedback_pct`). For `regulated_goods`, hard-drop any listing from a seller with <95% positive feedback or <50 total feedbacks — compliance risk is too high.

4. **VAT handling.** eBay UK listings denominate in GBP-inclusive-of-VAT by default. Emit `vat_inclusive: 1` unless the listing title explicitly states "ex VAT" / "plus VAT" / business-only pricing. For `match_archetype = "standard_class_led"` regulated goods where a matching listing omits VAT status, record `vat_inclusive: null` and add a warning rather than assume.

5. **Shipping & "free delivery".** When the listing says "Free delivery to United Kingdom" set `includes_shipping: 1`. Otherwise record the stated delivery cost in `evidence.shipping_cost_gbp_pence` but leave `includes_shipping: 0` — downstream landed-cost adds it in.

## Outputs

Same `result.json` + `run-manifest.md` contract as Alibaba. Key invariants:

- `source_id: "ebay"`
- `source_name: "eBay"` on every `supplier_quotes[]` entry
- `price_signal_role: "benchmark_ceiling"` (UK retail ceiling, not a floor)
- `currency: "GBP"` (typical; record otherwise when the listing differs)
- `region: "UK"`
- `validator_overlay: "validation/overlay-customs-classification"` in run metadata when the validator consumes a canary or production result

## L5 Baseline Binding

eBay is the already-operational UK retail benchmark source for L5. It remains `promotion_stage: stable` and contributes ceiling evidence only; it does not become an executable wholesale quote or an import-floor substitute. The standard six-file production skill directory is authoritative for promotion-gate checks:

- `SKILL.md`
- `result.schema.json`
- `checkpoint.schema.json`
- `canary.inputs.jsonl`
- `golden.assertions.yaml`
- `validator-rubric.yaml`

The calibration mirror under `tests/fixtures/calibration/marketplace-search/ebay/` must stay byte-identical for canary inputs and golden assertions. Any future behavior-affecting change must update `worker_behavior_version`, `gate_b_exact_slice_key`, the canary set, and the mirrored assertions in one bead.

## Failure Codes

Same set as Alibaba minus `SOURCE_AUTH_STATE_INVALID` (anonymous search is permitted on eBay), plus:

- `AUCTION_LISTING_SKIPPED` — informational warning only, not a hard failure
- `LOW_REPUTATION_SELLER_DROPPED` — counted in `hard_rejected_count`, not a hard failure

## Resume Strategy, Pagination, Canary, Run Manifest

Identical to `supplier-price-enrichment/alibaba` — see that file for full text. Checkpoint by profile_id + wave, maxPages 3, maxScrollCycles 2, canary limits to max 2 waves and 1 detail fetch, manifest frontmatter unchanged.
