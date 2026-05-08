---
skill_id: supplier-price-enrichment/1688
skill_version: 2026.04.21-1
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
validator_overlay_version: 2026.04.21-1
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: local_state
promotion_stage: candidate
worker_behavior_version: supplier-price-enrichment/1688@2026.04.21-1+1.0.0
validator_behavior_version: 2026.04.21-1
version_change_affects_behavior: true
gate_a_family_key: supplier-price-enrichment
gate_a_family_prequalified: false
gate_b_exact_slice_key: supplier-price-enrichment/1688::1688::supplier-price-enrichment/1688@2026.04.21-1+1.0.0::2026.04.21-1
gate_b_exact_slice_ready: false
maxPages: 3
maxScrollCycles: 2
browser_attach_mode: cdp_persistent_profile
browser_profile_path: $HOME/.local/share/govt-scraping/chrome-profiles/1688
---

# supplier-price-enrichment/1688

Searches 1688.com (Chinese wholesale — Alibaba Group's domestic-China sibling) for product pricing. Shares the structural contract with `supplier-price-enrichment/alibaba`; this file specifies only what differs. Read both files when executing — the Alibaba skill is the canonical spec for the shared sections.

## Use when
Use for `candidate_type IN ('standard_commodity','brand_family','regulated_goods')` when the dispatcher routes to `source_id = "1688"`. 1688 is typically the lowest-cost floor for mainland-China products; results land as `price_signal_role: "optimistic_floor"` (same as Alibaba) and are NOT executable without a sourcing agent.

## Do not use when
Do not use for UK-sourced products, brand-family candidates when brand is UK/EU-only, or any `candidate_type` other than the three listed above. Do not use for auth recovery (`auth-refresh/1688`) or health canaries (`source-health/1688`).

## Browser Attach Protocol (MANDATORY)

Identical to `skills/supplier-price-enrichment/alibaba/SKILL.md` with the source-specific substitutions below. Every constraint in the Alibaba skill's attach protocol — Chrome 136+ refusing CDP on the default profile, `--profile Default` / `--auto-connect` / `--cdp <port>` forbidden, `--headed` mandatory, single `batch` execution, `agent-browser close --all` recovery — applies here verbatim.

- **Profile path:** `$HOME/.local/share/govt-scraping/chrome-profiles/1688` (distinct from Alibaba — 1688 cookies / locale preferences are separate).
- **Landing URL:** `https://www.1688.com/` (not alibaba.com).
- **Search URL template:** `https://s.1688.com/selloffer/offer_search.htm?keywords=<urlencoded_query>` (not `/trade/search`).
- **Authenticated-header tell:** look for `我的阿里` (My Alibaba/1688) / 用户名 (username) markers in the header snapshot, plus absence of `登录` (login) / `免费注册` (free registration) clickables. Anonymous state here trips the same `SOURCE_AUTH_STATE_INVALID` short-circuit as Alibaba.
- **Expected locale:** `zh-CN`. Currency is **RMB (CNY)**. Shipping context is mainland China — NOT UK. Hard-drift if `locale_state != "zh-CN"` or `currency_state != "CNY"` after login — 1688's UI does not serve `English-GBP`.

## Inputs

Same `job_parameters` shape as `supplier-price-enrichment/alibaba` — the daemon builds it once in `run_enrich_pricing` and hands the same SearchProfile batch to whichever per-source skill the dispatcher picked. You receive:

- `source_id: "1688"`
- `search_profiles: [...]` — each entry MUST be read in full (head_noun, candidate_type, match_archetype, critical_attributes_json, numeric_specs_json, standards_json, noun_aliases_en, **noun_aliases_zh when populated**, preferred_terms, excluded_terms, query_waves_json, recall_risk_score_bps).
- `per_candidate_budget: {max_query_waves, shortlist_size, max_detail_fetches}`.

**Chinese-language handling — 1688-specific:**

1688 is a Chinese-language marketplace. Title matching, attribute comparison, and unit parsing MUST prefer Chinese tokens:

- If the SearchProfile has `noun_aliases_zh`, use those as primary query tokens for waves 0 and 1. `noun_aliases_en` is a fallback for when the lexicon lacks a zh entry.
- When the `query_waves_json` entries only have `query_en`, translate the semantic head noun to the closest Chinese equivalent for the wave-0 query (e.g. `stairlift` → `楼梯升降机` / `座椅电梯`). Record the translation mapping in the manifest Evidence section.
- Unit codes in 1688 listings use Chinese glyphs: `件` (piece), `个` (each), `只` (unit / pair-ish), `双` (pair), `套` (set), `箱` (carton), `包` (pack), `卷` (roll), `瓶` (bottle), `桶` (drum), `台` (machine). Normalize these to the SearchProfile's expected `quantity_unit` before emitting `quoted_quantity_unit`.

## Per-candidate execution protocol

Same 6-step flow as `supplier-price-enrichment/alibaba`: walk waves (0 → 1 → 2 → 3, plus 4 for regulated_goods), hard-conflict filter on head_noun / required critical_attrs / numeric-spec tolerance / excluded_terms / standards, diversify shortlist (max 2/seller, max 2/wave, max 1/title-family), detail-page enrichment on top 3, dual confidence scoring, emit supplier_quotes.

1688-specific adjustments:

- **Wave-2 market framing** uses Chinese framing tokens (`工业`, `商用`, `厂家直销`) instead of English `industrial commercial`.
- **Wave-4 compliance-off** for `regulated_goods`: drop standard token from query, post-filter against `standards_json` using both GB/T (Chinese) and BS EN (UK) equivalents when the research doc's standard-equivalence table applies.
- **Country-of-origin defaulting:** 1688 listings virtually all ship from CN; only record `country_of_origin` when the listing explicitly states otherwise.
- **FX conversion:** emit `unit_price_original_minor` in RMB fen (smallest unit), `currency: "CNY"`, plus `unit_price_gbp_pence` converted via the daemon-supplied FX rate. When FX is missing, leave `unit_price_gbp_pence` NULL rather than silently coercing.

## Outputs

Same `result.json` + `run-manifest.md` contract as Alibaba. Key invariants:

- `source_id: "1688"`
- `source_name: "1688"` on every `supplier_quotes[]` entry (not `"Alibaba"`)
- `price_signal_role: "optimistic_floor"` (1688 is China-wholesale floor)
- `currency: "CNY"`
- `region: "CN"`

## Failure Codes

Same set as Alibaba (`SOURCE_AUTH_STATE_INVALID`, `SOURCE_CAPTCHA_BLOCKED`, `LOCALE_OR_CURRENCY_DRIFT`, `ZERO_RESULTS_ALL_WAVES`, `HARD_CONFLICT_ZERO_SURVIVORS`, `CDP_PROFILE_LOCK_CONFLICT`, `AGENT_STALLED`, `AGENT_RUN_FAILED`, `PAGE_SHAPE_DRIFT`), plus one 1688-specific:

- `VOCABULARY_MISMATCH_EN_ONLY` — the SearchProfile had no `noun_aliases_zh`, no tender-doc Chinese translation, and the English head_noun produced zero results on 1688 across all waves. Emitted instead of `ZERO_RESULTS_ALL_WAVES` so operators know to extend the lexicon rather than assume the product isn't available.

## Resume Strategy, Pagination, Canary, Run Manifest

Identical to `supplier-price-enrichment/alibaba` — see that file for full text. Per-profile checkpointing via `last_completed_profile_id` + `last_completed_wave`, profile dir is the durable browser state, maxPages 3, maxScrollCycles 2, canary limits to max 2 waves and no detail fetches beyond the first listing, run manifest frontmatter fields identical.
