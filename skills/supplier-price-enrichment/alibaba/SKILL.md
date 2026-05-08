---
skill_id: supplier-price-enrichment/alibaba
skill_version: 2026.04.21-1
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
validator_overlay_version: 2026.04.21-1
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: local_state
promotion_stage: candidate
worker_behavior_version: supplier-price-enrichment/alibaba@2026.04.21-1+1.0.0
validator_behavior_version: 2026.04.21-1
version_change_affects_behavior: true
gate_a_family_key: supplier-price-enrichment
gate_a_family_prequalified: false
gate_b_exact_slice_key: supplier-price-enrichment/alibaba::alibaba::supplier-price-enrichment/alibaba@2026.04.21-1+1.0.0::2026.04.21-1
gate_b_exact_slice_ready: false
maxPages: 3
maxScrollCycles: 2
browser_attach_mode: cdp_persistent_profile
browser_profile_path: $HOME/.local/share/govt-scraping/chrome-profiles/alibaba
---

# supplier-price-enrichment/alibaba

## Use when
Use this skill when the daemon needs Alibaba price signals for one or more qualified `product_candidate`s. The daemon invokes this skill with a batch of `search_profiles` as `job_parameters.search_profiles[]` — one profile per candidate — plus a `source_id = "alibaba"` marker.

## Do not use when
Do not use this skill for authentication recovery (that is `auth-refresh/alibaba`), source health canaries (`source-health/alibaba`), 1688 searches (`supplier-price-enrichment/1688`), or any non-Alibaba marketplace. Do not use it when the profile's `search_eligibility_state` is `skip` — the daemon already filters those out upstream.

## Browser Attach Protocol (MANDATORY — reuse from auth-refresh/alibaba)

This skill MUST attach via `agent-browser --profile "$HOME/.local/share/govt-scraping/chrome-profiles/alibaba" --headed batch ...`. Every constraint in `skills/auth-refresh/alibaba/SKILL.md` (Chrome 136+ refusing CDP on the default profile, `--profile Default` spawning a throwaway dir, `--auto-connect` / `--cdp <port>` forbidden, anti-detection flags forbidden, single-batch execution required, `--headed` mandatory because headless gets slider-CAPTCHA-intercepted on `/trade/*` paths) applies here verbatim. Do not re-derive or relax those rules.

If `agent-browser` reports `⚠ --profile, --headed ignored: daemon already running`, run `agent-browser close --all` then retry the batch — the first successful canary run validated that this is safe and necessary.

Before the first query wave, verify the session is authenticated by issuing `open https://www.alibaba.com/`, `snapshot -i -c`, and inspecting for a `Sign in` / `Create account` clickable. If not authenticated, emit `failure_code: SOURCE_AUTH_STATE_INVALID` and return immediately — do not try to search anonymously. Auth repair is `auth-refresh/alibaba`'s job, not this skill's.

## Inputs

`job_parameters` structure (rendered into your prompt as JSON):

```json
{
  "source_id": "alibaba",
  "search_profiles": [
    {
      "id": "sp-<uuid>",
      "product_candidate_id": "pc-<uuid>-lot-1-candidate-<slug>",
      "head_noun": "chair",
      "candidate_type": "standard_commodity",
      "match_archetype": "dimension_material_led",
      "critical_attributes_json": { "back_type": { "value": "mesh", "criticality": "required" } },
      "numeric_specs_json": { "seat_width_mm": { "nominal": 480, "min": 460, "max": 500, "unit": "mm", "criticality": "preferred" } },
      "standards_json": ["BS EN 1335-2:2018"],
      "noun_aliases_en": ["chair", "task chair", "office chair"],
      "preferred_terms": ["ergonomic", "adjustable"],
      "query_waves_json": [
        {"wave": 0, "strategy": "exact_anchor",    "query_en": "chair mesh", ...},
        {"wave": 1, "strategy": "alias_swap",       "query_en": "task chair mesh", ...},
        {"wave": 2, "strategy": "market_frame",     "query_en": "industrial commercial chair", ...},
        {"wave": 3, "strategy": "broad_existence",  "query_en": "chair", ...}
      ],
      "recall_risk_score_bps": 3500,
      "search_eligibility_state": "auto_search"
    }
  ],
  "per_candidate_budget": {
    "max_query_waves": 3,
    "shortlist_size": 8,
    "max_detail_fetches": 3
  }
}
```

**You MUST read the full `search_profiles[]` array verbatim. Do not collapse it into `head_noun + candidate_name`. The structured fields (`critical_attributes_json`, `numeric_specs_json`, `standards_json`) drive hard-conflict filtering on search results — dropping them will produce irrelevant quotes.**

## Per-candidate execution protocol

For each `search_profile` in `search_profiles[]`:

### 1. Walk query waves in order (precise → broad)

Execute waves `0`, `1`, `2`, `3` in that sequence using the pre-computed `query_en` text. Stop early when a wave yields enough shortlisted results to satisfy `per_candidate_budget.shortlist_size`. For `candidate_type: "regulated_goods"`, wave `4` (compliance_off) is a recovery retry if waves 0-3 yield zero usable results — use standards as a post-filter rather than a query term.

Each wave is a URL navigation:
```
https://www.alibaba.com/trade/search?SearchText={urlencode(wave.query_en)}
```

### 2. Snapshot and hard-conflict filter

After each wave's search, `snapshot -i -c` the results page. For each listing in the snapshot:

- **Head-noun conformance:** the listing title MUST contain the `head_noun` or one of `noun_aliases_en`. Drop otherwise.
- **Required critical attributes:** every `critical_attributes_json` entry with `criticality: "required"` MUST match verbatim or by a tender-consistent substitute (e.g. "IP65" also accepts "IP67"). Drop otherwise.
- **Numeric-spec tolerance:** every `numeric_specs_json` entry with `criticality: "required"` MUST have listing-reported values inside the `[min, max]` window. When the listing does not surface a spec, degrade evidence to `missing` but do not hard-drop — flag for detail-page fetch.
- **Exclusion terms:** if the listing title contains any `excluded_terms`, drop unconditionally.
- **Standards:** record each `standards_json` entry as `evidence_state = match | missing | ambiguous | conflict` based on title + structured-attribute presence. For `regulated_goods`, a `conflict` is a hard drop; for other categories it is a confidence penalty.

### 3. Diversified shortlist

From the post-filter survivors, shortlist up to `per_candidate_budget.shortlist_size` listings with **diversity constraints**:
- max 2 listings per seller
- max 2 listings per wave
- max 1 listing per normalized title family (collapse minor price/pack variants)

### 4. Detail-page enrichment

For the top `per_candidate_budget.max_detail_fetches` shortlisted listings, `open <listing_url>` + `snapshot -c` and extract:
- structured specs: dimensions, material, power, standards, MOQ, unit price, pack size
- seller metadata: name, country, years active, certifications
- shipping terms and incoterms (if stated)
- compliance evidence tier: `structured_attribute` (explicit spec field) > `title_body_mention` (keyword in free text) > `absent`

### 5. Per-listing confidence scoring

For each retained listing, compute two scores (report both in bps, 0-10000):

- **match_confidence_bps** = 0.40 · product_type_match + 0.30 · critical_attrs_match + 0.20 · numeric_specs_match + 0.10 · evidence_quality. Measures *is this the right product?*
- **quote_usability_confidence_bps** = 0.35 · unit_clarity + 0.25 · MOQ_clarity + 0.20 · shipping_clarity + 0.20 · completeness. Measures *is this quote usable for downstream gross-margin gating?*

### 6. Emit supplier_quotes

For each retained listing, write one row to `result.json` (daemon persists to `supplier_quotes`):

```json
{
  "product_candidate_id": "pc-...",
  "search_attempt_id": "sa-...",
  "source_name": "Alibaba",
  "source_url": "https://www.alibaba.com/product-detail/...",
  "source_listing_id": "<alibaba_product_id>",
  "region": "CN",
  "quote_class": "organic_listing",
  "price_signal_role": "optimistic_floor",
  "unit_price_original_minor": 1075,
  "currency": "CHF",
  "unit_price_gbp_pence": 950,
  "quoted_quantity": 1,
  "quoted_quantity_unit": "piece",
  "minimum_order_quantity": 50,
  "ship_from_country": "CN",
  "country_of_origin": "CN",
  "incoterm": "FOB",
  "listing_pack_qty": 1,
  "listing_pack_unit": "piece",
  "is_used_or_refurbished": 0,
  "is_ready_to_ship": 1,
  "vat_inclusive": 0,
  "compliance_evidence_tier": "title_body_mention",
  "match_confidence_bps": 7200,
  "quote_usability_confidence_bps": 6800,
  "query_wave_number": 0,
  "evidence_states_json": {
    "back_type": "match",
    "seat_width_mm": "missing",
    "BS EN 1335-2:2018": "ambiguous"
  },
  "evidence": "{\"listing_title\":\"...\",\"seller\":\"...\",\"snippets\":[...]}",
  "valid_until": "2026-05-21T00:00:00Z"
}
```

Alibaba results are ALWAYS `price_signal_role: "optimistic_floor"` — they feed the gross-margin gate as a wholesale floor, never as the executable quote. Do not emit `price_signal_role: "executable"` even if the listing claims ready-to-ship.

## Outputs

- `result.json` — machine-readable contract matching `skills/supplier-price-enrichment/alibaba/result.schema.json`. Contains `search_attempts[]` (one per wave executed) and `supplier_quotes[]` (one per retained listing).
- `run-manifest.md` — narrative trace with evidence anchors to listing URLs, query text, and the session transcript location.

Both files are MANDATORY every run. Silent omission is a compliance failure. When the batch fails (CAPTCHA, auth issue, zero results across all waves), still emit `result.json` with `supplier_quotes: []`, populated `search_attempts[]` describing what was tried, and a non-null `failure_code` — don't return empty files.

## Non-goals

- Authentication recovery or CAPTCHA solving (use `auth-refresh/alibaba`).
- Source-health monitoring (use `source-health/alibaba`).
- Cross-source correlation, relation resolution, or gross-margin computation — those are downstream daemon stages.
- Negotiation uplift estimation or buyer outreach.
- Persisting directly to SQLite — the daemon owns authoritative writes; you emit a JSON contract only.

## Failure Codes

- AGENT_AUTH_CHALLENGE
- SOURCE_AUTH_STATE_INVALID
- SOURCE_CAPTCHA_BLOCKED
- SOURCE_SOFT_BLOCKED
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- ZERO_RESULTS_ALL_WAVES
- HARD_CONFLICT_ZERO_SURVIVORS
- CDP_PROFILE_LOCK_CONFLICT
- AGENT_STALLED
- AGENT_RUN_FAILED

## Resume Strategy

Each `search_profile` within a batch is independently resumable via the checkpoint — record `last_completed_profile_id` + `last_completed_wave` in the checkpoint payload after each wave. On resume, skip profiles already fully completed and re-enter the partially-completed profile at `last_completed_wave + 1`. The persistent Chrome profile is the durable browser state; do not attempt to replay cookies via `--state <json>`.

## Pagination Bounds
- `maxPages`: 3 (per wave, per profile)
- `maxScrollCycles`: 2

## Canary

Verify that one search_profile with `search_eligibility_state: "auto_search"` and a realistic `head_noun` (e.g. `"led work light"`) produces at least one shortlisted result with non-null `match_confidence_bps` and `unit_price_gbp_pence`, without triggering CAPTCHA or authentication failure. The canary MUST NOT exceed `per_candidate_budget.max_query_waves = 2` and MUST NOT fetch detail pages beyond the first listing.

## Run Manifest Instructions

You must produce a run manifest that the daemon can finalize into the durable `run-manifest.md` artifact.

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
- items_processed
- items_skipped
- items_failed
- warnings_count
- agent_claimed_pages_visited
- agent_claimed_source_url
- agent_claimed_fingerprint_state
- agent_claimed_locale_state
- agent_claimed_currency_state

Required narrative sections:
- Summary
- Steps Taken
- Decisions Made
- Warnings
- Evidence

Major claims MUST be anchored to observable evidence: listing URLs, query text per wave, rendered seller names, citation to the agent-browser transcript file. Do not claim a match, a confidence score, or a diversity decision without an evidence anchor.
