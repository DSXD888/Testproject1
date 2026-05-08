# Stage 9 — Supplier Price-Signal Match: Delivery Manifest

**Date:** 2026-04-21
**Author:** Claude (Opus 4.7, 1M context)
**Status:** Plumbing complete; agent loop validated on one source (Alibaba) end-to-end via PTY; DB persistence of supplier_quotes pending.

---

## 1. Intent

Stage 9 is **Price-Signal Match** — the pipeline step after product extraction (stage 7) and quoteability scoring (stage 8). It drives agent-based marketplace searches (Amazon / eBay / Alibaba / 1688) and writes `supplier_quotes` rows that downstream gross-margin / landed-cost stages consume. Before this session, stage 9 was declared (stage name registered, `AdmissionPriority::SupplierEnrichment` enum present, `search_profiles` / `supplier_quotes` / `search_attempts` / `listing_evidence` tables migrated) but **not connected**: no code projected ProductCandidate → SearchProfile, no dispatcher existed, no source-specific skill existed. The rich extraction schema at stage 7 was isolated — nothing downstream consumed it.

The delivery documented here closes the gap end-to-end for one source (Alibaba) and provides sibling skills + routing for 1688, eBay, and Amazon.

## 2. Pre-state (verified 2026-04-21 AM)

| Artifact | Pre-state |
|---|---|
| `qualification_runs` | 0 rows (qualify stage had never run for this workspace) |
| `product_candidates` | 545 rows, all with `candidate_type='sku'` (invalid enum), `match_archetype='direct_match'` (invalid enum), `material/brand_model/critical_attributes/numeric_specs` all NULL |
| `search_profiles` | 0 rows; table structurally complete but no writer |
| `supplier_quotes` | 0 rows; table structurally complete but no writer |
| `enrichment_skill_path_for()` | did not exist |
| `skills/supplier-price-enrichment/*` | directory did not exist |
| `--enrich-pricing` CLI flag | did not exist |
| `auth-refresh/alibaba` / `source-health/alibaba` | existed, `promotion_stage: stable`, but with no CDP mandate (pre-Chrome-136 spec) |
| `auth-refresh/1688` | same gap — no CDP mandate |

## 3. What was built

### Pass 1 — initial "fix it" (stage 8.5 + stage 9 + Alibaba)

**Rust — stage 8.5 (`crates/daemon/src/main.rs`):**

- `strongest_search_anchor(brand_model, critical_attrs, numeric_specs, standards)` — picks the single strongest identity hook for wave-0 query generation. Priority: brand+model → first `required`-criticality critical attribute → first numeric spec → first standards token.
- `build_query_waves_json(head_noun, candidate_name, candidate_type, brand_model, material, critical_attrs, numeric_specs, standards, preferred_terms)` — produces the 5-wave ladder (exact_anchor → alias_swap → market_frame → broad_existence → compliance_off) as JSON. Wave 4 is only populated for `regulated_goods`.
- `compute_quoteability_score_bps(...)` — deterministic rules-based score per research doc §8. Additive bonuses for structured slots present; penalties for custom fabrication, installed goods, regulated-goods-with-missing-compliance, closed access routes.
- `compute_recall_risk_bps(...)` — deterministic risk score. Drives agent shortlist budget (low risk → top 8; high → top 20).
- `search_eligibility_state(quoteability_bps)` — `>=6500 → auto_search`, `>=4500 → manual_only`, `<4500 → skip`.
- `derive_noun_aliases_en(head_noun, preferred_terms)` — lexicon placeholder that seeds from head_noun + preferred terms.
- `run_build_search_profiles_stage(state_db, source_id)` — reads product_candidates where `qualification_runs.goods_qualified=1 AND no existing search_profile`, computes scores + waves, writes the `search_profiles` row, mirrors `quoteability_score_bps` + `recall_risk_score_bps` back onto the candidate.
- Wired into `run_deterministic_pipeline` between qualify_lot and relate stages. `--pipeline <source>` now logs `build-search-profiles stage complete search_profiles_built=N`.

**Rust — stage 9 dispatcher (`crates/daemon/src/main.rs`):**

- `enrichment_skill_path_for(source_id)` — match on source, returns skill path. Added `alibaba` arm in pass 1.
- `source_name_for_source_id(source_id)` — canonical capitalization for `supplier_quotes.source_name`.
- `run_enrich_pricing(runtime, source_id, limit, cli)` — SELECTs eligible search_profiles (`search_eligibility_state='auto_search' AND no supplier_quotes yet`), builds `job_parameters` with the full SearchProfile JSON inlined (head_noun, candidate_type, match_archetype, critical_attributes_json, numeric_specs_json, standards_json, noun_aliases, preferred_terms, query_waves, recall_risk_score_bps), invokes via `run_bound_skill` which goes through the existing PTY path.
- `--enrich-pricing <source_id>` CLI flag + `--enrich-pricing-limit <N>` (default 4).
- Dispatch branch in `main()` between `--pipeline` and `--run-next-job`.

**Skill — supplier-price-enrichment/alibaba:**

- `SKILL.md` (~250 lines): references `auth-refresh/alibaba` for the CDP + persistent profile protocol; mandates explicit "read the full search_profiles[] array verbatim, don't collapse to head_noun + name"; 6-step execution protocol (walk waves → snapshot + hard-conflict filter → diversified shortlist → detail-page enrichment → dual confidence scoring → emit supplier_quotes); 10+ failure codes; `price_signal_role: "optimistic_floor"` invariant.
- `result.schema.json` (~150 lines): schema for `{source_id, batch_summary, search_attempts[], supplier_quotes[], warnings[], extraction_metadata}`. Every `supplier_quotes[]` entry constrained to `source_name: "Alibaba"` + `price_signal_role: "optimistic_floor"`.
- `checkpoint.schema.json`: per-profile resume state.

### Pass 2 — multi-source expansion (1688 / eBay / Amazon + auth-refresh parity + routing + dry-run)

**Three sibling skills** (`skills/supplier-price-enrichment/{1688,ebay,amazon}/`):

Each is a compact sibling that references the Alibaba skill as canonical for shared sections (attach protocol, 6-step execution, output contract, resume strategy) and specifies only what differs per source:

- **1688:** Chinese wholesale. Profile path `~/.local/share/govt-scraping/chrome-profiles/1688`. Search URL `https://s.1688.com/selloffer/offer_search.htm?keywords=...`. Authenticated tell: `我的阿里` / `登录`. Locale `zh-CN`, currency `CNY`, FX-converted to GBP. noun_aliases_zh preferred; Chinese market framing (`工业`, `商用`, `厂家直销`); Chinese unit normalization (`件`, `个`, `只`, `双`, `套`, `箱`, `包`, `卷`, `瓶`, `桶`, `台`). Added failure code `VOCABULARY_MISMATCH_EN_ONLY`. `price_signal_role: "optimistic_floor"`.
- **eBay UK:** Profile path `~/.local/share/govt-scraping/chrome-profiles/ebay`. Search URL `https://www.ebay.co.uk/sch/i.html?_nkw=...&LH_PrefLoc=1` (UK sellers only). **Anonymous search permitted** — does NOT short-circuit on unauthenticated session (unlike Alibaba). Auction listings skipped. Seller-reputation gate for regulated goods (≥95% positive, ≥50 feedbacks). VAT-inclusive default. Failure codes: `AUCTION_LISTING_SKIPPED`, `LOW_REPUTATION_SELLER_DROPPED`. `price_signal_role: "benchmark_ceiling"`.
- **Amazon UK:** Profile path `~/.local/share/govt-scraping/chrome-profiles/amazon`. Search URL `https://www.amazon.co.uk/s?k=...&language=en_GB&__mk_en_GB=...`. `--headed` non-negotiable (most aggressive anti-bot of the four). CAPTCHA detection → `SOURCE_CAPTCHA_BLOCKED` short-circuit (routes to `auth-refresh/amazon` which does not exist yet — noted as open dependency). Variant-ASIN disambiguation required; no family-level "from £X" pseudo-prices. Fulfillment class (`Amazon-fulfilled`/`Prime`/`FBM`) recorded. Failure codes: `AMAZON_BUSINESS_LOGIN_REQUIRED`, `VARIANT_ASIN_AMBIGUOUS`. `price_signal_role: "benchmark_ceiling"`.

Each has `result.schema.json` (copy of Alibaba's with `source_id`, `source_name`, `price_signal_role` constants patched) and `checkpoint.schema.json` (copy).

**auth-refresh/1688 upgrade:**

- Added Browser Attach Protocol section matching `auth-refresh/alibaba`: Chrome 136+ rationale, forbidden-flag list, canonical attach command, session-validity probe using Chinese markers (`我的阿里`, `登录`, `滑块`), operator re-login workflow emitted as `1688_MANUAL_LOGIN_REQUIRED` with verbatim step-by-step for the human.
- Version bumped `2026.03.30-1 → 2026.04.21-1`. `promotion_stage: stable → candidate`. Frontmatter gains `browser_attach_mode: cdp_persistent_profile` + `browser_profile_path`.
- Failure codes extended with `1688_MANUAL_LOGIN_REQUIRED` and `CDP_PROFILE_LOCK_CONFLICT`.

**Rust — routing expansion:**

- `enrichment_skill_path_for()` gained `1688` / `ebay` / `amazon` arms.
- `supplier_sources_for_candidate_type(candidate_type) -> &'static [&'static str]` — routing matrix per research doc §4.3:
  - `exact_sku → [amazon, ebay]` (brand+model UK-discoverable)
  - `brand_family → [amazon, ebay, alibaba]`
  - `standard_commodity → [amazon, ebay, alibaba, 1688]`
  - `regulated_goods → [amazon, ebay]` (compliance more trustworthy UK-side)
  - `bundle → [alibaba, 1688]`
  - `installed_goods`/`custom_or_fabricated`/`service_not_applicable`/`unknown → []`
- Currently advisory; `run_enrich_pricing` still dispatches to one source named on the CLI. Hooking this into the dispatcher is a ~20-line follow-up.

**Rust — dry-run flag:**

- `--enrich-pricing-dry-run` — resolves the skill path, builds `job_parameters`, emits both to stdout as JSON, and exits without invoking the agent. Used to validate routing + inspect exactly what the agent would receive in its prompt.

## 4. File inventory

| Path | Change | Approx. lines |
|---|---|---|
| `crates/daemon/src/main.rs` (stage 8.5 block + helpers) | added | ~350 |
| `crates/daemon/src/main.rs` (stage 9 dispatcher + dry-run + routing matrix) | added | ~200 |
| `crates/daemon/src/main.rs` (CLI flags `--enrich-pricing`, `--enrich-pricing-limit`, `--enrich-pricing-dry-run`) | added | ~25 |
| `crates/daemon/src/main.rs` (dispatch branch in main) | added | ~10 |
| `skills/supplier-price-enrichment/alibaba/SKILL.md` | new | ~250 |
| `skills/supplier-price-enrichment/alibaba/result.schema.json` | new | ~150 |
| `skills/supplier-price-enrichment/alibaba/checkpoint.schema.json` | new | ~15 |
| `skills/supplier-price-enrichment/1688/SKILL.md` | new | ~95 |
| `skills/supplier-price-enrichment/1688/result.schema.json` | new | ~150 (copy + patch) |
| `skills/supplier-price-enrichment/1688/checkpoint.schema.json` | new | ~15 |
| `skills/supplier-price-enrichment/ebay/SKILL.md` | new | ~100 |
| `skills/supplier-price-enrichment/ebay/result.schema.json` | new | ~150 (copy + patch) |
| `skills/supplier-price-enrichment/ebay/checkpoint.schema.json` | new | ~15 |
| `skills/supplier-price-enrichment/amazon/SKILL.md` | new | ~120 |
| `skills/supplier-price-enrichment/amazon/result.schema.json` | new | ~150 (copy + patch) |
| `skills/supplier-price-enrichment/amazon/checkpoint.schema.json` | new | ~15 |
| `skills/auth-refresh/1688/SKILL.md` | rewritten (added Browser Attach Protocol section, version bump, promotion_stage→candidate) | net +80 |

Build: `cargo check -p daemon` clean after every change.

## 5. Contracts

### Input to every supplier-price-enrichment skill

The daemon dispatcher (`run_enrich_pricing`) builds exactly this shape and passes it verbatim to `run_bound_skill` → `PromptRenderer::render` → Claude's `-p <prompt>` argument. No lossy projection anywhere between DB and prompt.

```json
{
  "source_id": "alibaba" | "1688" | "ebay" | "amazon",
  "search_profiles": [
    {
      "id": "sp-<uuid>",
      "product_candidate_id": "pc-<uuid>-lot-<n>-candidate-<slug>",
      "candidate_name": "<full product description>",
      "head_noun": "<semantic head>",
      "candidate_type": "exact_sku | standard_commodity | brand_family | bundle | custom_or_fabricated | installed_goods | regulated_goods | service_not_applicable | unknown",
      "match_archetype": "standard_class_led | dimension_material_led | electrical_spec_led | pack_count_led | compatibility_led | model_exact | unknown",
      "critical_attributes_json": { "<slot>": { "value": "...", "criticality": "required|preferred|descriptive|exclusion" } },
      "numeric_specs_json":       { "<spec>": { "nominal": N, "min": N, "max": N, "unit": "...", "criticality": "..." } },
      "standards_json":           [ "<standard>", ... ],
      "noun_aliases_en":          [ "<alias>", ... ],
      "preferred_terms":          [ "<term>", ... ],
      "excluded_terms":           [ "<term>", ... ] | null,
      "query_waves_json": [
        { "wave": 0, "strategy": "exact_anchor",   "query_en": "...", "filters_hint": {...} },
        { "wave": 1, "strategy": "alias_swap",      "query_en": "...", "filters_hint": {...} },
        { "wave": 2, "strategy": "market_frame",    "query_en": "...", "filters_hint": {...} },
        { "wave": 3, "strategy": "broad_existence", "query_en": "...", "filters_hint": {...} },
        { "wave": 4, "strategy": "compliance_off",  "query_en": "...", "filters_hint": {...} }  /* regulated_goods only */
      ],
      "recall_risk_score_bps": 0..10000,
      "search_eligibility_state": "auto_search"
    }
  ],
  "per_candidate_budget": {
    "max_query_waves": 3,
    "shortlist_size": 8,
    "max_detail_fetches": 3
  },
  "notice_ids": ["<notice-id>", ...]
}
```

### Output from every supplier-price-enrichment skill

`result.json` conforming to the per-source `result.schema.json`. Shape:

```json
{
  "source_id": "<source>",
  "batch_summary": { "profiles_processed", "profiles_skipped", "profiles_failed", "total_waves_executed", "total_listings_shortlisted", "total_detail_pages_fetched" },
  "search_attempts": [ { "product_candidate_id", "search_profile_id", "wave_number", "strategy", "query_text", "locale", "result_count", "hard_rejected_count", "shortlisted_count", ... } ],
  "supplier_quotes":  [ { "product_candidate_id", "search_attempt_id", "source_name", "source_url", "source_listing_id", "region", "quote_class", "price_signal_role", "unit_price_original_minor", "currency", "unit_price_gbp_pence", "match_confidence_bps", "quote_usability_confidence_bps", "evidence", ... } ],
  "warnings": [ "..." ],
  "extraction_metadata": { "skill_version", "browser_profile_path", "session_transcript_path", "batch_started_at", "batch_completed_at", "failure_code" }
}
```

Plus `run-manifest.md` with the standard frontmatter + Summary / Steps Taken / Decisions Made / Warnings / Evidence sections.

## 6. Validation history

| When | Command | Outcome |
|---|---|---|
| pass 1 | `cargo check -p daemon` | clean |
| pass 1 | Seeded synthetic rich product_candidate for "364_26 Stairlifts, Steplifts, Vertical Through Floor Lifts and Hoists" (notice `pc-01698761-…`): candidate_type=standard_commodity, match_archetype=standard_class_led, head_noun=stairlift, 4 critical_attributes with criticality markers, 3 numeric_specs with tolerance windows (user_weight 160-180-200kg, rail_length 2.5-3.5-4.5m, speed 0.1-0.15-0.2 m/s), standards `["BS EN 81-40:2020"]`, 4 EN aliases, 2 excluded terms | row inserted |
| pass 1 | `cargo run --bin daemon -- --pipeline procontract` | stage 8.5 generated 1 search_profile, eligibility=auto_search, quoteability=8500, recall_risk=2000, 4-wave ladder with correct queries (`stairlift straight-rail`, `industrial commercial stairlift aluminium rail, powder-coated steel carriage`, `stairlift`) |
| pass 1 | `cargo run --bin daemon -- --enrich-pricing alibaba --enrich-pricing-limit 1` | daemon dispatched, spawned Claude CLI with PTY, Claude loaded `supplier-price-enrichment/alibaba` skill, attached via CDP to `~/.local/share/govt-scraping/chrome-profiles/alibaba`, ran session-validity probe, detected anonymous session (no login completed yet), correctly short-circuited per skill contract with `failure_code: SOURCE_AUTH_STATE_INVALID`, emitted schema-conformant `result.json` (supplier_quotes:[], search_attempts:[], warnings populated, extraction_metadata.failure_code set) + full `run-manifest.md`, released browser, daemon exit 0. `agent_session_logs` row 126 of 126 persisted. 6 min walltime. |
| pass 2 | `cargo check -p daemon` after multi-source expansion | clean |
| pass 2 | `--enrich-pricing {alibaba,1688,ebay,amazon} --enrich-pricing-dry-run --enrich-pricing-limit 1` | All four resolved to the correct skill path; all four received the full SearchProfile JSON (head_noun, candidate_type, critical_attributes_json with 4 slots, numeric_specs_json with 3 specs, all 4 query_waves, recall_risk_score_bps, preferred_terms, excluded_terms). Verified per-source routing is not lossy. |
| pass 2 | `--enrich-pricing nonexistent-source --enrich-pricing-dry-run` | errored cleanly: `no supplier-price-enrichment skill registered for source 'nonexistent-source' — add one under skills/supplier-price-enrichment/ and register it in enrichment_skill_path_for()` |

## 7. How to use

### Trigger the deterministic projection (stage 8.5)

```bash
cargo run --bin daemon -- --pipeline <source_id>
```

Runs extract_lots → prequalify_notice → qualify_lot → **build_search_profiles** → relate. Logs `build-search-profiles stage complete search_profiles_built=N`.

### Trigger supplier enrichment (stage 9) for one source

```bash
# Real run (spawns Claude + agent-browser, ~5-10 min/profile)
cargo run --bin daemon -- --enrich-pricing alibaba --enrich-pricing-limit 4

# Dry-run (prints the job_parameters JSON + skill path, no agent)
cargo run --bin daemon -- --enrich-pricing alibaba --enrich-pricing-dry-run --enrich-pricing-limit 1
```

Valid source_id values today: `alibaba`, `1688`, `ebay`, `amazon`. Any other value fails fast with a pointer to `enrichment_skill_path_for()`.

### One-time per-source login (unblocks the green path)

```bash
# Alibaba
agent-browser --profile "$HOME/.local/share/govt-scraping/chrome-profiles/alibaba" \
  --headed open https://login.alibaba.com/
# then pin shipping country = United Kingdom in My Alibaba settings

# 1688
agent-browser --profile "$HOME/.local/share/govt-scraping/chrome-profiles/1688" \
  --headed open https://login.1688.com/

# Amazon UK
agent-browser --profile "$HOME/.local/share/govt-scraping/chrome-profiles/amazon" \
  --headed open https://www.amazon.co.uk/ap/signin

# eBay UK (optional — anonymous search permitted but logged-in is more stable)
agent-browser --profile "$HOME/.local/share/govt-scraping/chrome-profiles/ebay" \
  --headed open https://signin.ebay.co.uk/
```

After each login, `agent-browser close --all` to release the profile lock.

## 8. Open gaps (known, intentional)

1. **`supplier_quotes` DB persistence is not wired.** `run_bound_skill` has persistence branches for `procurement-ingestion/*` (line 3744) and `pdf-extraction/*` (line 3756) but no `supplier-price-enrichment/*` branch. `result.json` is written to the session workdir correctly but not promoted into `supplier_quotes`. Closing this is ~30-50 lines: a new `persist_supplier_quotes_from_agent_output(state_db, skill_id, result_json)` + a match arm in `run_bound_skill`.

2. **Multi-source dispatch per candidate is advisory only.** `supplier_sources_for_candidate_type()` maps each candidate_type to an ordered source list, but `run_enrich_pricing` still dispatches to a single CLI-named source. Hooking this in means: for each auto_search profile, enumerate sources via the matrix, enqueue one dispatcher invocation per (source, profile) pair — or batch by source.

3. **`source-health/{1688,ebay,amazon}` canaries do not exist.** Only `source-health/alibaba` exists. Mirroring it is mechanical — copy + patch URLs, locale, authentication markers.

4. **`auth-refresh/{ebay,amazon}` do not exist.** Referenced by the new skills' failure-handling paths (e.g. Amazon's `SOURCE_CAPTCHA_BLOCKED` recommends routing to `auth-refresh/amazon`). Until they land, operators must resolve challenges manually via a headed session.

5. **Lexicon is a placeholder.** `head_noun_family`, `market_frame_en`, `market_frame_zh`, `noun_aliases_zh` columns on `search_profiles` are all NULL today. The deterministic generator uses `preferred_terms` as its alias feed. Research doc §5.4 describes the full lexicon deliverable; the plumbing works without it but recall on the alias_swap + market_frame waves is weaker.

6. **Only `standard_commodity` path exercised.** Four archetypes (`exact_sku`, `brand_family`, `bundle`, `regulated_goods`) have different hard-conflict rules in their respective skill flows. No real-data validation of those paths yet.

7. **One real run only.** The synthetic stairlift SearchProfile drove one Alibaba dispatch that correctly short-circuited on auth. No run has produced actual `supplier_quotes` yet. The green-path unblocker is the one-time login in §7.

## 9. Design notes worth remembering

- **Chrome 136+ CDP restriction** is the load-bearing constraint on this whole subsystem. Chrome refuses `--remote-debugging-port` on its default user-data-dir. Every supplier-search skill must use a dedicated persistent profile at a non-default path. The profile paths under `~/.local/share/govt-scraping/chrome-profiles/<source>/` are the durable browser state — the skills do not re-derive them.
- **`--headed` is MANDATORY for Alibaba and Amazon**, HIGHLY RECOMMENDED for 1688, and OPTIONAL for eBay. This is empirical: a 2026-04-21 canary proved `--headless=new` Chrome is slider-CAPTCHA-intercepted on Alibaba `/trade/*` paths even from a warm dedicated profile, while the same profile in `--headed` mode reached listings cleanly.
- **No anti-detection flags**. The skills explicitly forbid `--disable-blink-features=AutomationControlled` and similar. Our anti-bot posture is logged-in state, not stealth flags.
- **Price-signal role is source-specific and non-swappable.** Alibaba / 1688 = `optimistic_floor` (wholesale floor, never executable). Amazon / eBay UK = `benchmark_ceiling` (UK retail ceiling). Downstream gross-margin gating uses these differently; do not merge them.
- **Per-source failure codes are additive to the shared set.** Each skill defines the shared codes (CDP_PROFILE_LOCK_CONFLICT, SOURCE_CAPTCHA_BLOCKED, LOCALE_OR_CURRENCY_DRIFT, etc.) plus source-specific ones (`VOCABULARY_MISMATCH_EN_ONLY` on 1688, `AUCTION_LISTING_SKIPPED` on eBay, `VARIANT_ASIN_AMBIGUOUS` on Amazon). Keep this pattern when adding sources.
- **The dispatcher renders the full SearchProfile, not a summary.** `run_enrich_pricing` builds `job_parameters.search_profiles[]` from the DB rows unchanged — `critical_attributes_json`, `numeric_specs_json`, `standards_json`, `query_waves_json` all travel as full nested JSON into the prompt. Any future refactor that substitutes a summary or stringified form here would silently regress the extraction-precision work at stage 7.

## 10. Related docs

- `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` §4.3 (source routing matrix), §5.4 (SearchProfile shape + lexicon), §5.7 (price-signal matching data flow), §8 (quoteability scoring)
- `skills/auth-refresh/alibaba/SKILL.md` — canonical Browser Attach Protocol + operator re-login workflow (referenced by all four enrichment skills)
- `skills/supplier-price-enrichment/alibaba/SKILL.md` — canonical per-candidate execution protocol (referenced by 1688 / eBay / Amazon siblings)
- `migrations/0001_sqlite_state_initial.sql` lines 146-185 (`product_candidates`), 187-217 (`search_profiles`), 219-280 (`supplier_quotes`), 282-310 (`search_attempts`), 312-340 (`listing_evidence`)
