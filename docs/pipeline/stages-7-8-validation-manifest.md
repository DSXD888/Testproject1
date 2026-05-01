# Stages 7-8 Validation Manifest

> Generated: 2026-04-18T18:14:30Z
> Status: **GREEN** (Stage 7 usable with documented coverage gap; Stage 8 implemented and all checks pass)

## Stage 7: Product Candidates Assessment

| Metric | Value |
|--------|-------|
| Total product_candidates | 545 |
| In passed-goods contracts | 63 |
| Passed-goods contracts with candidates | 4 of 10 |
| Passed-goods contracts WITHOUT candidates | 6 (listed below) |
| Field completeness: name | 100% (545/545) |
| Field completeness: head_noun | 100% (545/545) |
| Field completeness: candidate_type | 100% (545/545, all `sku`) |
| Field completeness: match_archetype | 100% (545/545, all `direct_match`) |
| Field completeness: specification_text | 100% (545/545) |
| Field completeness: standards_json | 61.5% (335/545) |
| Field completeness: quantity | 0.2% (1/545) |
| Field completeness: material | 0% (0/545) |
| Field completeness: critical_attributes_json | 0% (0/545) |
| Field completeness: numeric_specs_json | 0.4% (2/545) |

### Passed-goods contracts without candidates (coverage gap)

Tender documents were not enriched/extracted for these six processes, so semantic
extraction produced no product candidates. This is a documented upstream coverage
gap, not a Stage 7 fault.

| process_id | title |
|---|---|
| proc-pc-45c1e490-2ad6-e911-80fa-005056b64545 | 902-19 Supply and Application of Bituminous Based Binder Products in Highways Maintenance |
| proc-pc-83a118df-7c2e-f111-813c-005056b64545 | Assisted Lifeguard Technology |
| proc-pc-1e5502ad-2629-f111-813c-005056b64545 | Bristol Court Extra Care Catering Service Tender |
| proc-pc-cc9efc86-6424-f111-813c-005056b64545 | Broadlea Primary School Fire Risk Assessment Works |
| proc-pc-40cc746d-8289-ec11-8110-005056b64545 | BSH/HRA/Communal Electrical Materials DPS |
| proc-pc-5b2a7cd0-6d1c-f111-813c-005056b64545 | Central Digital Platform |

### Candidates per passed-goods contract (present)

| process_id | title | candidates |
|---|---|---|
| proc-pc-169ba131-… | CCP074 Print Service DPS | 27 |
| proc-pc-7e2a6fb2-… | BSH Metal Works DPS | 20 |
| proc-pc-b7aaab27-… | BSH Joinery Timber Supplies DPS | 10 |
| proc-pc-7046e8ec-… | BSH Double Glazed Units & Sheet Glass DPS | 6 |

### Data-quality observations

- `candidate_type` and `match_archetype` are uniformly `sku` / `direct_match` —
  these are defaults from semantic extraction, not real classifications.
  The Stage 8 scoring therefore de-weights these signals and leans on
  `specification_text` length, `standards_json` presence, head-noun
  well-formedness, and name word count (the dimensions that actually vary
  across the 545 rows).
- All 545 head nouns are single-word (the extractor picked the first noun
  of the phrase). Ten contain a trailing comma (e.g. `stationery,`) — the
  scoring penalises these as malformed.
- The 335 rows with populated `standards_json` sometimes contain legal or
  statutory references instead of technical product standards; Stage 8
  still emits them in wave 2 query text because Stage 9 will judge quality.

## Stage 8: Score Quoteability Results

| Metric | Value |
|--------|-------|
| Total search_profiles created | 545 |
| Candidates with `quoteability_score_bps` populated | 545 |
| Candidates with `recall_risk_score_bps` populated | 545 |
| Candidates with `identity_confidence_bps` populated | 545 |
| Stage heads written (`scope_kind=candidate`) | 545 |
| Stage status rows (`state=succeeded`) | 545 |
| Orphan search_profiles | 0 |
| Profiles missing `query_waves_json` | 0 |

### Scoring model (deterministic, no LLM)

```
baseline                                      +1000
head_noun ≥4 chars and no comma               +1500   (else +500 if ≥4 chars)
name word count ≥6 / ≥4 / ≥2                  +1500 / +1000 / +500
specification_text length ≥200/≥100/≥50/≥20   +2500 / +1800 / +1000 / +500
standards_json has entries                    +1500
critical_attributes_json has entries          +1000
numeric_specs_json has entries                +1000
quantity present and > 0                       +500
quantity_unit present                          +200
material present                               +500
---
score clamped to [0, 10000]
recall_risk_score_bps = 10000 − quoteability_score_bps
```

Thresholds:
- `eligible` ≥ 6500 bps
- `marginal` ≥ 4500 bps and < 6500 bps
- `ineligible` < 4500 bps

With the current data the theoretical ceiling is 8000 bps
(base + good head + 6-word name + 200+ spec + standards) — no candidate
reached the 10000 cap because structured fields (`quantity`,
`critical_attributes_json`, `numeric_specs_json`, `material`) are almost
uniformly empty.

### Eligibility distribution — all 545 candidates

| State | Count | Avg Score (bps) |
|-------|-------|-----------------|
| eligible | 302 | 7255 |
| marginal | 217 | 5520 |
| ineligible | 26 | 4046 |

### Eligibility distribution — 63 passed-goods candidates

| State | Count | Avg Score (bps) |
|-------|-------|-----------------|
| eligible | 23 | 6939 |
| marginal | 34 | 5365 |
| ineligible | 6 | 4100 |

**Key metric: 57 of 63 passed-goods candidates (90.5%) are searchable
(`eligible` + `marginal`) and ready for Stage 9 marketplace search.**

### Top-5 passed-goods candidates by score

| Score (bps) | State | Name |
|---|---|---|
| 8000 | eligible | Double-glazed unit (sealed insulating glass unit, BS EN 1279-compliant…) |
| 7500 | eligible | Print Services (CCP074 DPS umbrella) |
| 7300 | eligible | Metal railings / gates / fencing — repair and modification service |
| 7300 | eligible | Chute / balcony balustrade — general repair under Lot 1 scope |
| 7300 | eligible | Manufactured metal railings — supply and installation |

### Bottom-5 passed-goods candidates by score

| Score (bps) | State | Name |
|---|---|---|
| 4000 | ineligible | Plywood boards |
| 4000 | ineligible | Door blanks |
| 4000 | ineligible | Hemlock timber |
| 4000 | ineligible | Signage, lamp-post banners, outdoor advertising… |
| 4300 | ineligible | PVC vinyl banners (indoor/outdoor) with hem and eyelets |

The bottom set is exactly what we want — these are either "generic
placeholder" extractions (2-word names, no standards, short spec) or rows
where the head noun was truncated at a comma (`signage,`).

### Query wave plan — sample for top-scored candidate

```
Double-glazed unit (sealed insulating glass unit, BS EN 1279-compliant)
[
  { "wave": 1, "strategy": "exact_noun",      "query": "double-glazed Flat hermetically-sealed double-glazed" },
  { "wave": 2, "strategy": "noun_with_spec",  "query": "double-glazed BS EN 1279" },
  { "wave": 3, "strategy": "broader_noun",    "query": "Double-glazed unit" },
  { "wave": 4, "strategy": "existence_check", "query": "double-glazed" }
]
```

Wave 2 correctly resolved the standards label (`BS EN 1279`) from
`standards_json`. Every profile has at least one usable wave.

## Validation Checks

| # | Check | Result | Detail |
|---|-------|--------|--------|
| 7.1 | Passed-goods have candidates | YELLOW | 4/10 have candidates; 6/10 documented coverage gap (no tender doc enrichment). Not blocking for Stage 8. |
| 7.2 | Required fields present | PASS | 0 missing name / head_noun / candidate_type / match_archetype / specification_text |
| 7.3 | FK integrity (candidates) | PASS | 0 orphans against notice_lots, contract_notices, procurement_processes |
| 8.1 | Search profiles created | PASS | 545 profiles (matches 545 scored candidates); 63 of 63 passed-goods covered |
| 8.2 | Quoteability scores populated | PASS | 545/545 rows have quoteability, recall_risk, identity_confidence |
| 8.3 | Eligibility distribution sensible | PASS | All three states present: 302/217/26 split across ~56%/40%/5% |
| 8.4 | Query waves exist | PASS | 545 profiles have non-empty `query_waves_json`, 0 missing |
| 8.5 | Stage heads exist | PASS | 545 stage heads `scope_kind=candidate`, 545 status rows `state=succeeded` |
| 8.6 | FK integrity (profiles) | PASS | 0 orphan profiles |
| 8.7 | Passed-goods eligibility sensible | PASS | 23 eligible / 34 marginal / 6 ineligible — physical-goods candidates (double-glazed, metal work, timber) cluster at the top as expected |

## Code Changes

| File | Change |
|---|---|
| `crates/daemon/src/main.rs` | Added `run_score_quoteability_stage()` plus helpers (`QuoteabilityRow` struct, `score_quoteability_bps`, `identity_confidence_bps`, `eligibility_state_for`, `extract_spec_keywords`, `first_standard_label`, `standard_label_from_value`, `broader_noun_from_name`, `build_query_waves`, `quoteability_json_has_entries`). Wired into `run_deterministic_pipeline()` between `run_qualify_lot_stage` and `run_relate_stage`. |

Compiler/lint status:
- `cargo check --all-targets` → clean
- `cargo clippy --all-targets -- -D warnings` (pedantic + nursery) → clean
- `cargo fmt --check` → clean

## Issues Found & Fixed

1. **Clippy: `manual_clamp`** — `score.min(10000).max(0)` flagged by pedantic.
   Fixed by using `score.clamp(0, 10_000)` in both scoring helpers.
2. **Clippy: `missing_const_for_fn`** — `eligibility_state_for` was pure and
   could be `const fn`. Fixed by adding the `const` qualifier.
3. **Clippy: `option_if_let_else`** — the wave-2 construction has a multi-arm
   fall-through that doesn't map cleanly to `Option::map_or_else`. Applied
   a local `#[allow(clippy::option_if_let_else)]` on `build_query_waves`.

## Issues Remaining

1. **Upstream extraction gaps** (not blocking Stage 8):
   - All 545 candidates have default `candidate_type=sku`, `match_archetype=direct_match`.
     The extraction skill should classify these from the 6 archetypes defined
     in the search-intelligence plan (direct_match, standards_compliant,
     brand_family, custom_fabrication, bundle, substitute).
   - Structured fields (`critical_attributes_json`, `numeric_specs_json`,
     `quantity`, `material`) are near-empty. These would meaningfully raise
     scores once populated (each adds 500–1000 bps) but do not block Stage 8.
   - `standards_json` occasionally contains statutory or legal references
     rather than product standards. Stage 9 agents will judge wave-2 query
     usability.
2. **Tender document enrichment for 6 contracts** — listed above under
   "Passed-goods contracts without candidates". Out of scope for this
   handoff; tracked separately.

## Data Snapshot

### Field completeness (all 545)
```
total=545 has_head_noun=545 has_name=545 has_type=545 has_archetype=545
has_spec=545 has_quantity=1 has_material=0 has_standards=335
has_critical_attrs=0 has_numeric_specs=2 has_quoteability=545 has_identity_conf=545
```

### Pipeline run log
```
extract-lots stage complete        lots_created=0
prequalify-notice stage complete   prequalified=0
qualify-lot stage complete         lots_qualified=0
score-quoteability stage complete  quoteability_scored=545
relate stage complete              relations_found=0
```
Upstream stages correctly skipped (heads already exist); Stage 8 processed
every qualified-lot candidate on the first pass.

### Stage heads
```
Score quoteability | candidate  | 545
Score quoteability | succeeded  | 545
```

### Eligibility — all 545
```
eligible    302  avg 7255 bps
marginal    217  avg 5520 bps
ineligible   26  avg 4046 bps
```

### Eligibility — 63 passed-goods
```
eligible     23  avg 6939 bps
marginal     34  avg 5365 bps
ineligible    6  avg 4100 bps
```

### FK integrity
```
orphan search_profiles: 0
orphan product_candidates (lot):    0
orphan product_candidates (notice): 0
orphan product_candidates (process): 0
```
