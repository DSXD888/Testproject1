# Full Pipeline Re-Run Manifest

> Generated: 2026-04-21 19:30 UTC (initial)
> Updated:   2026-04-21 21:15 UTC (truly final — semantic drain completed; all 3 drainers
>            and marketplace loop exited cleanly)
> Status: GREEN (re-run accomplished primary objective of unlocking more data through the pipeline)

## Changes Made

### 1. Expiry threshold: 30 days → 14 days
- File: `crates/daemon/src/main.rs` (`candidate_deadline_check`)
- Controls ProContract interest-registration pre-filter.
- The four contracts rejected as expired under the 30-day threshold in the
  baseline were truly past deadline (2026-04-14/15 vs today 2026-04-21), so
  they remain expired. The value of the change is in *future* ingestion where
  contracts with deadlines in the 14-30 day window will now get interest
  registered instead of skipped.

### 2. CPV parser rewrite (root cause fix for "REVIEW: unknown category")
- File: `crates/daemon/src/main.rs` (prequalify-notice stage, CPV extraction)
- ProContract's newer HTML-scraped payloads changed shape vs earlier ingestion:
  - `categories` is nested under `detail.` (not at root)
  - separator is `\n` (not `|`)
  - some payloads use a JSON array of strings instead of a single string
- Parser now checks `$.categories` and `$.detail.categories`, splits on both
  `|` and `\n`, and handles array-valued categories.
- **Impact:** goods-qualified contracts jumped from 10 → 22 (+12), and
  non-DPS goods-qualified contracts from 5 → 18 (+13) — this is what actually
  unlocked the "more data" goal.

### 3. Claim-by-id fix for `--ingest` flows
- Files: `crates/daemon/src/jobs/claim.rs`, `crates/daemon/src/jobs/runner.rs`,
  `crates/daemon/src/main.rs`
- Added `JobRunner::claim_job_by_id` that claims a specific pending job by id
  rather than FIFO `claim_next_job`. Ingest call sites (contracts-finder,
  procontract/procontract-interest/procontract-enrich, marketplace-search, the
  test fixture) now use this to claim the job they just enqueued, instead of
  being preempted by older pending extraction/validation jobs.
- **Impact:** eliminated "claimed unexpected workflow job" errors that were
  blocking marketplace search while extraction jobs were draining. This was
  the thing that unlocked running drainers and marketplace search in
  parallel on the same queue.

## Data Delta (baseline → current)

| Table                         | Before | After | Delta |
|-------------------------------|--------|-------|-------|
| contract_notices              | 103    | 133   | +30   |
| procurement_processes         | 103    | 133   | +30   |
| notice_lots                   | 141    | 179   | +38   |
| qualification_runs            | 244    | 308   | +64   |
| product_candidates            | 545    | 820   | **+275** |
| search_profiles               | 545    | 820   | **+275** |
| search_attempts               | 76     | 193   | **+117** |
| supplier_quotes               | 5      | 10    | **+5** |
| document_extractions          | 202    | 348   | +146  |
| document_semantic_extractions | 108    | 166   | +58   |
| tender_document_sets          | ~12    | 18    | +6    |
| raw_documents (with storage)  | ~169   | 318   | +149  |

## Qualification Funnel (post re-run)

| Outcome                      | Count |
|------------------------------|-------|
| PASSED: goods                | 22    |
| REVIEW: unknown category     | 31    |
| REJECTED: services/works     | 69    |
| REJECTED: expired            | 11    |

Baseline was 10 PASSED: goods. **+12 goods-qualified contracts** from the CPV
parser fix alone. `REJECTED: expired` grew from 4 to 11 because the full
prequalify reset pulled in older notices whose deadlines had passed.

## Non-DPS Goods Contracts (18 total, vs 5 in baseline)

| Title | Candidates | Searchable | Quotes |
|-------|-----------:|-----------:|-------:|
| 25112 Library Services Engage, Design and Fit (Ely) | 65 | 57 | **2** |
| CPU 8309 Building Materials-Heavy | 10 | 10 | **3** |
| 218_26 Specialist Vehicle Contract Hire/Leasing | 40 | 34 | 0 |
| 107_26 Inspection and Maintenance of Indoor Gymnasium | 44 | 11 | 0 |
| Asset Management System | 7 | 7 | 0 |
| 364_26 Stairlifts, Steplifts, Vertical Through Floor Lifts | 0 | 0 | 0 |
| Bristol Court Extra Care Catering Service Tender | 0 | 0 | 0 |
| 902-19 Supply and Application of Bituminous Based Binder | 0 | 0 | 0 |
| Central Digital Platform | 0 | 0 | 0 |
| 2526-0430 Domestic Gas and Ad hoc Non-domestic Boiler | 0 | 0 | 0 |
| 274_26 Grounds Maintenance Machinery | 0 | 0 | 0 |
| Assisted Lifeguard Technology | 0 | 0 | 0 |
| 16_26 Flooring (Supply, Fit and Refurbishment) | 0 | 0 | 0 |
| 1206 - Postgraduate Research Management System | 0 | 0 | 0 |
| 191_28 Electricity | 0 | 0 | 0 |
| Broadlea Primary School Fire Risk Assessment Works | 0 | 0 | 0 |
| 301_26 The supply of Liquid Fuels | 0 | 0 | 0 |
| 509_27 Parking Management Solutions | 0 | 0 | 0 |

Verified: all 13 non-DPS goods contracts with 0 candidates have also had
0 document_sets, 0 structural extractions, and 0 pending extraction jobs.
Their `interest_registration` stage has completed, so they are waiting on
buyers to respond with document access before enrichment can pull them
in. A future `--ingest procontract-enrich` run will pick them up as
buyers respond (no further daemon action needed now).

## Key Metrics

- **Non-DPS goods contracts:** 18 (baseline 5)
- **Non-DPS non-service searchable candidates:** 119 (baseline 0)
- **Non-DPS goods contracts with quotes:** 2 (CPU 8309 + Library Services Ely, baseline 0)
- **Total supplier quotes:** 10 (baseline 5) — **+5 new quotes**: 3 Amazon-UK + 2 eBay on 3 new candidates
- **Candidates with any quote:** 4 (baseline 1)
- **Quote price roles:** uk_retail_benchmark (eBay 5 + Amazon 3), cn_supplier_floor (Alibaba 2)

### Quoted Candidates (all 4)

| Candidate | Source Contract | Quotes |
|-----------|-----------------|-------:|
| Double-glazed unit (BS EN 1279, argon-filled) | BSH/HRA DPS Sheet Glass (baseline) | 5 |
| Aggregates (framework-wide, detailed grades in ITT) | CPU 8309 Building Materials-Heavy (**new**) | 3 |
| Library carpet / floor covering (Ely Library) | 25112 Library Services (**new**) | 1 |
| Ely Library carpet / flooring package | 25112 Library Services (**new**) | 1 |

## Issues Found & Fixed

1. **ProContract payload format drift** (silent regression)
   - New payloads nested `categories` under `detail` and used `\n` separator
     and occasional JSON-array form. Old parser looked only at root-level
     `$.categories` split on `|`, so every new notice ended up
     `legal_category='unknown'` and got filtered out of the funnel.
   - Fixed in the CPV extraction helper; re-ran prequalify for all 133
     notices and got 22 goods-qualified (vs 10 before).

2. **`claim_next_job` FIFO vs just-enqueued ingest jobs**
   - `--ingest` flows enqueue a job then call `claim_next_job`, which returns
     the oldest pending. With ~145 extraction jobs already queued, new
     ingest jobs were never claimed and always hit
     `ensure!(claimed.id == job_id, "claimed unexpected workflow job")`.
   - Added `claim_job_by_id` and pointed all six ingest call sites at it.
     Extraction drainers keep using `claim_next_job` so ordering still holds
     for them. Ingest/marketplace search now runs in parallel with
     extraction drainers on the same DB.

3. **Stale sync_source jobs left over from failed enrichment retries**
   - 2 stale sync_source jobs (1 running, 1 pending) from earlier
     "claimed unexpected workflow job" aborts. Manually cancelled with a
     last_error note. No downstream effect — the enrichment work itself had
     already succeeded on the first call.

## Background Work (all completed by end of session)

- **Semantic extraction drain: COMPLETE.** All 88+ queued semantic jobs
  finished. 3 drainers on runtime/runtime2/runtime3 exited cleanly on
  "expected a pending workflow job" (queue empty).
- **Marketplace search: COMPLETE** (12-iteration bash loop). Ran 12 batches
  across ebay-uk / amazon-uk / gem-imports / mx-wholesale. 36 candidate
  slots processed, 5 new quotes landed on 3 new candidates.
- **Alibaba search (`alibaba-search`)** was not run in this session because
  it requires a CDP-connected Chrome profile. A future run of
  `--ingest alibaba-search` against the 119 eligible non-DPS candidates
  would likely add cn_supplier_floor quotes (only 2 existed at baseline).
- **Still pending for future runs:** 13 of the 18 non-DPS goods contracts
  have 0 candidates because the ProContract buyer hasn't yet responded to
  interest requests. Future `--ingest procontract-enrich` calls will
  discover these when they appear in the "Recently Added" tab.

## Pipeline Stage Consistency

All stage_status rows are `succeeded` or `completed` (no `failed` entries
that would block re-runs). Stage heads are present for every notice at
every expected stage.

| Stage                  | heads | status distribution         |
|------------------------|------:|-----------------------------|
| Normalize              | 133   | 164 succeeded               |
| prequalify notice      | 133   | 133 succeeded               |
| extract lots           | 123   | — (10 notices have no lots) |
| qualify lot            | 175   | 175 succeeded               |
| Score quoteability     | 580   | 580 succeeded               |
| relate                 | 133   | 133 succeeded               |
| price_signal_match     | 32    | 37 succeeded                |
| acquire docs           | 17    | 53 succeeded                |
| tender_enrichment      | 18    | 39 completed                |
| interest_registration  | 131   | 24 completed + 107 skipped  |

## FK Integrity

All clean:

| Check                             | Count |
|-----------------------------------|------:|
| orphan_candidates (no lot)        | 0     |
| orphan_quotes (no candidate)      | 0     |
| orphan_attempts (no candidate)    | 0     |

## Build Health

- `cargo check --all-targets`: clean
- `cargo clippy --all-targets -- -D warnings`: clean
- `cargo build --release --bin daemon`: clean
- Three code changes (expiry threshold, CPV parser, claim-by-id) all present
  on `main` branch working tree.
