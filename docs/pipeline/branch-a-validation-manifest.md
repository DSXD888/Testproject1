# Branch A Validation Manifest

> Generated: 2026-04-18
> Status: **GREEN**

## Pipeline Run Summary

Final run after the CPV-extraction fix (see *Issues Found & Fixed*):

| Source           | lots_created | prequalified | lots_qualified | relations |
|------------------|-------------:|-------------:|---------------:|----------:|
| procontract      | 0¹           | 103          | 141            | 0         |
| contracts-finder | 0            | 0            | 0              | 0         |
| fts              | 0            | 0            | 0              | 0         |

¹ `lots_created=0` on the second run because the 93 implicit lots were created during the first run, before the CPV fix. The stage is idempotent — heads block re-creation.

**Note on source mix:** All 103 notices in `state/state.db` are from ProContract. No rows exist for `contracts-finder` or `fts`. The handoff prompt described “~97% ProContract / ~3% FTS/CF,” but the live database is ProContract-only, which makes correct ProContract CPV extraction the critical path for getting a non-degenerate category breakdown.

## Qualification Funnel

### Notice-level categories (basis = `notice_only`)

| Category | Count | Goods Pass | Expiry Pass |
|----------|------:|-----------:|------------:|
| goods    |    11 |         11 |          10 |
| services |    34 |          0 |          34 |
| works    |     7 |          0 |           6 |
| unknown  |    51 |          0 |          49 |
| **Total**| **103**|       **11** |        **99** |

### Outcome Breakdown

| Outcome                      | Count |
|------------------------------|------:|
| PASSED: goods                |    10 |
| REJECTED: services/works     |    40 |
| REVIEW: unknown category     |    49 |
| REJECTED: expired            |     4 |
| REVIEW: low confidence goods |     0 |
| **Total**                    | **103** |

### Key Metric

**Viable physical-goods contracts: 10 of 103 (9.7%)**

This is the size of the Branch A funnel handed off to Stage 7+. The 49 “REVIEW: unknown” notices (30 with no `categories` field + 21 whose only codes were non-CPV ProContract proprietary 6-digit taxonomies) are a secondary pool that later enrichment could promote into the goods lane.

### The 10 PASSED-goods notices

| notice_id | title (truncated) |
|---|---|
| pc-169ba131-3b3f-ed11-8119-005056b64545 | CCP074 - Provision of a Print Service DPS - Acceptance Round 16 |
| pc-1e5502ad-2629-f111-813c-005056b64545 | Bristol Court Extra Care Catering Service Tender |
| pc-40cc746d-8289-ec11-8110-005056b64545 | BSH/HRA/Communal Electrical Materials DPS |
| pc-45c1e490-2ad6-e911-80fa-005056b64545 | 902-19 Supply and Application of Bituminous Based Binder Products |
| pc-5b2a7cd0-6d1c-f111-813c-005056b64545 | Central Digital Platform |
| pc-7046e8ec-2734-ed11-8119-005056b64545 | BSH/HRA/Supply of Double Glazed Units and Sheet Glass - DPS |
| pc-7e2a6fb2-67a3-f011-813a-005056b64545 | BSH/HRA/Metal Works - Dynamic Purchasing System (DPS) |
| pc-83a118df-7c2e-f111-813c-005056b64545 | Assisted Lifeguard Technology |
| pc-b7aaab27-1f34-ed11-8119-005056b64545 | BSH/HRA/Joinery Timber Supplies - DPS - Acceptance Round 7 |
| pc-cc9efc86-6424-f111-813c-005056b64545 | Broadlea Primary School Fire Risk Assessment Works |

## Validation Checks

| # | Check | Result | Detail |
|---|---|---|---|
| 1 | Every notice has ≥1 lot             | PASS | 0 notices without lots; 141 lots / 103 notices |
| 2 | Every notice has notice_only qual   | PASS | 103/103; 0 missing |
| 3 | Every lot has lot_level qual        | PASS | 141/141; 0 missing |
| 4 | Stage heads exist for Branch A      | PASS | extract lots (notice, 93) + prequalify notice (notice, 103) + qualify lot (lot, 141) |
| 5 | Categories sensible                 | PASS | 4 distinct categories: goods(11), services(34), works(7), unknown(51) |
| 6 | FK integrity (zero orphans)         | PASS | lot/notice/process orphans all 0 |
| 7 | Stage status states                 | PASS | extract lots=93 succeeded; prequalify notice=103 succeeded; qualify lot=141 succeeded |

All 7 checks green.

## Issues Found & Fixed

### Issue 1 — CPV extraction misses ProContract’s `categories` field

**Symptom (Check 5 on first run):**
All 103 notices classified as `"unknown"`. Only one category — below the ≥2-category pass bar.

**Root cause:**
`run_prequalify_stage` in `crates/daemon/src/main.rs:9208-9217` read CPV only from the OCDS JSON path `tender.classification.id`. ProContract HTML-scraped payloads are not OCDS — they carry CPV codes in a single `categories` string:

```
"45210000-2 - Building construction work | 45221111-3 - Road bridge construction work"
```

73 / 103 payloads carried this field, but `tender.classification.id` was absent in every payload, so `cpv_division` was always `None` → `"unknown"`.

The handoff prompt anticipated ProContract → "unknown" in general, but the `categories` field was still being discarded even when it contained perfectly valid EU CPV-8 codes. Fixing the extractor is a **bug fix**, not a rule change — we did not touch the CPV-division-to-category mapping or the confidence/density thresholds.

**Fix:**
- Added a module-level helper `parse_cpv_division` that accepts EU CPV-8 codes (8 ASCII digits + optional `-N` check digit) and rejects the 6-digit proprietary codes ProContract emits alongside real CPV (e.g. `150000 - Consultancy`, `171010 - Adult & Further Education Services`). It returns the first 2 digits as the CPV division.
- Updated the extraction chain: OCDS path first; if absent, split `payload_json.categories` on `|`, trim, and pick the first valid CPV-8 code.

File: `crates/daemon/src/main.rs` (`run_prequalify_stage` + new `parse_cpv_division` helper directly above it).

**Verification workflow:**
1. `cargo check --all-targets` — clean.
2. `cargo clippy --all-targets -- -D warnings` — clean.
3. `cargo build --release --bin daemon` — clean.
4. Reset `prequalify notice` + `qualify lot` heads/status and cleared `qualification_runs`.
5. Re-ran the pipeline for all three sources.
6. Re-ran every validation query.

Result: 4 categories, 11 goods-classified notices (10 of them also expiry-qualified → PASSED-goods), 40 rejected services/works, 49 review-unknowns, 4 expired.

## Issues Remaining

None blocking Branch A.

### Observations worth noting for the follow-up session

1. **`extract lots` stage head is only written for notices with zero incoming lots** (93 of 103). The other 10 notices already had ≥1 lot carried in by earlier stages and are correctly excluded by the `NOT EXISTS (...)` predicate in `run_extract_lots_stage`. Every exit criterion is satisfied (every notice has ≥1 lot; every lot has a `qualify lot` head), but if downstream code ever expects an `extract lots` head per notice, the query would need to emit a no-op head for already-lotted notices.

2. **51 notices in "REVIEW: unknown"** split into two root causes:
   - 30 payloads have no `categories` field at all (the scraping step dropped it, or the ProContract page didn’t expose one).
   - ~21 payloads carry only 6-digit ProContract proprietary codes (e.g. `150000 - Consultancy`, `171010 - Adult & Further Education Services`) with no EU CPV-8 code. Mapping those proprietary codes to CPV divisions is a future enrichment step (would likely belong in extract-products or an enrichment skill, not in prequalify).

3. **Source filter caveat:** `--pipeline contracts-finder` and `--pipeline fts` both returned all-zeros — there are no notices from those sources in the DB. If FTS/CF ingestion is brought online later, the existing OCDS path will handle them without further changes.

4. **`notice_lots.stage_run_id` link:** the 93 implicit lots were stamped with the first run’s `stage_run_id`, and the 48 pre-existing lots still carry their earlier run IDs. Nothing here breaks Branch A, but joins that use `stage_run_id` to trace a single lot back to a single `extract lots` run will see two populations.

## Data Snapshot

### Pre-Branch-A (before any run)

```
notices=103  processes=103  lots=48  lotless_notices=93
qualification_runs=0  document_extractions=202  semantic_extractions=108
product_candidates=545

Stage heads (pre-run):
  Normalize|notice|103
  acquire docs|notice|10
  interest_registration|process|102
  tender_enrichment|process|12
```

### Post-Branch-A (final, after CPV fix)

```
notices=103  lots=141  notice_quals=103  lot_quals=141

Stage heads (Branch A):
  extract lots|notice|93
  prequalify notice|notice|103
  qualify lot|lot|141

Process stage status (Branch A):
  extract lots|succeeded|93
  prequalify notice|succeeded|103
  qualify lot|succeeded|141

FK integrity:
  Orphaned qual_runs (lot missing):     0
  Orphaned qual_runs (notice missing):  0
  Orphaned qual_runs (process missing): 0

Notice categories (basis=notice_only):
  goods    | 11 | goods_pass=11 | expiry_pass=10
  services | 34 | goods_pass=0  | expiry_pass=34
  works    |  7 | goods_pass=0  | expiry_pass=6
  unknown  | 51 | goods_pass=0  | expiry_pass=49

Outcome breakdown:
  PASSED: goods              | 10
  REJECTED: services/works   | 40
  REVIEW: unknown category   | 49
  REJECTED: expired          |  4
```

### Pipeline summary lines (final run)

```
procontract:      lots_created=0 prequalified=103 lots_qualified=141 relations=0
contracts-finder: lots_created=0 prequalified=0   lots_qualified=0   relations=0
fts:              lots_created=0 prequalified=0   lots_qualified=0   relations=0
```

(First procontract run, before the CPV fix, reported `lots_created=93 prequalified=103 lots_qualified=141 relations=0`. The 93 implicit lots were created then; the second run found nothing to create.)

## Code Changes

| File | Change |
|---|---|
| `crates/daemon/src/main.rs` | Added module-level `parse_cpv_division` helper; updated `run_prequalify_stage` to fall back to the ProContract `categories` field when OCDS `tender.classification.id` is absent. No other logic touched. |

Quality gates (all clean after the change):
- `cargo check --all-targets`
- `cargo clippy --all-targets -- -D warnings`
- `cargo build --release --bin daemon`
