# Stage 9 Validation Manifest — Marketplace Search

> Generated: 2026-04-19
> Status: **YELLOW** — all build-time verification GREEN; live-execution
> validation (running the 5 skills against real marketplaces for the 57
> searchable candidates) is a separate operational task that requires PTY
> sessions and API spend, and is not performed within this build session.

## Scope of this handoff

Stage 9 is the most complex pipeline stage: 5 separate agent skills, one per
marketplace, with candidate-driven dispatch and per-source pricing semantics.
The work delivered by this manifest is the **buildable infrastructure** —
skills, daemon wiring, persistence — all the way to the point where running
`target/release/daemon --ingest gem-imports` (or the other 4 sources) will
dispatch a real PTY session and persist real data.

What is NOT delivered here: the ~285 live PTY sessions (57 candidates ×
5 sources) that produce the actual `supplier_quotes` rows. Those runs
consume real-world wallclock time and Claude API spend and are expected to
happen over the operational life of the daemon, not inside a single build
session.

## Source Coverage

| Source | Skill Built | Daemon-registered | Attempts | Quotes Found | Avg Price (pence) |
|--------|-------------|-------------------|----------|-------------|-------------------|
| gem-imports    | YES | YES (`gem-imports`)    | 0 | 0 | n/a (no live runs yet) |
| mx-wholesale   | YES | YES (`mx-wholesale`)   | 0 | 0 | n/a |
| amazon-uk      | YES | YES (`amazon-uk`)      | 0 | 0 | n/a |
| ebay-uk        | YES | YES (`ebay-uk`)        | 0 | 0 | n/a |
| alibaba        | YES | YES (`alibaba-search`) | 0 | 0 | n/a |

### Skill package inventory (25 files, 5 × 5)

Every skill package under `skills/marketplace-search/{source}/` contains:

- `SKILL.md` — frontmatter + playbook (source-specific guidance for the
  sub-Claude running the PTY session)
- `result.schema.json` — JSON schema for the skill's emitted `result.json`
- `checkpoint.schema.json` — JSON schema for the continuation cursor
- `canary.inputs.jsonl` — a one-candidate canary input for rehearsal
- `golden.assertions.yaml` — invariants the validator overlay will check

All 10 schema files parse as valid JSON. All 5 canary inputs parse as valid
JSONL. (See `Data Snapshot` below for the validation commands used.)

### Price-signal role mapping

| Source          | `source_id`     | Registry `kind`        | `price_signal_role`      | `region` | `quote_class` |
|-----------------|-----------------|------------------------|--------------------------|----------|---------------|
| Gem Imports     | `gem-imports`   | `Supplier`             | `uk_wholesale`           | `GB`     | `wholesale`   |
| MX Wholesale    | `mx-wholesale`  | `Supplier`             | `uk_wholesale`           | `GB`     | `wholesale`   |
| Amazon UK       | `amazon-uk`     | `Supplier`             | `uk_retail_benchmark`    | `GB`     | `spot`        |
| eBay UK         | `ebay-uk`       | `Supplier`             | `uk_retail_benchmark`    | `GB`     | `spot`        |
| Alibaba         | `alibaba-search`| `Supplier`             | `cn_supplier_floor`      | `CN`     | `trade`       |

Note: the Alibaba *marketplace-search* source registers as `alibaba-search`
to keep its namespace distinct from the existing `alibaba` canary /
auth-refresh source (which points to `source-health/alibaba` and
`auth-refresh/alibaba`).

## Wholesale Discount Configuration

`DEFAULT_UK_RETAIL_WHOLESALE_DISCOUNT_BPS = 3000` (30 %) is declared in
`crates/daemon/src/main.rs`. The constant is operator-adjustable in the
2000..=4000 bps range without re-scraping. Stage 9 itself never applies this
discount — raw retail prices from Amazon / eBay land in
`supplier_quotes.unit_price_gbp_pence` unchanged and are tagged with
`price_signal_role = "uk_retail_benchmark"`. Downstream margin math
(Stage 10+) multiplies by `(10000 - bps) / 10000` when computing estimated
wholesale from a retail-benchmark quote.

## Quote Distribution

| Price Signal Role       | Count | Avg Price (pence) |
|-------------------------|-------|-------------------|
| uk_wholesale            | 0     | n/a               |
| uk_retail_benchmark     | 0     | n/a               |
| cn_supplier_floor       | 0     | n/a               |

(Expected to populate once live runs execute; see "Operational steps" below.)

## Margin Estimates

Margin estimation is a Stage 10+ concern; Stage 9 ends at the
`supplier_quotes` table. This section intentionally left empty — see
`Key Metric` below for the Stage-9-relevant coverage number.

## Key Metric

- **57 of 63 passed-goods candidates (90.5 %) are `eligible` or `marginal`**
  and therefore qualified to be searched. This number is unchanged from the
  Stage 7–8 manifest and is confirmed by the Stage 9 candidate-selection
  query (23 eligible + 34 marginal = 57).
- **0 of 57 searchable candidates have dual-source pricing today** — Stage 9
  machinery is built but no live runs have been executed inside this build
  session.

## Validation Checks

| # | Check | Result | Detail |
|---|-------|--------|--------|
| 9.1 | Search attempts recorded | N/A (no live runs) | Machinery in place: `persist_marketplace_search_results` writes one row per (candidate × wave executed) into `search_attempts`. |
| 9.2 | Supplier quotes found | N/A (no live runs) | Machinery in place: each `quotes[]` entry persists one `supplier_quotes` row with `unit_price_original_minor` (raw) + `unit_price_gbp_pence` (converted). |
| 9.3 | Price signal roles correct | PASS (static) | Each source's `price_signal_role` is set from the source registry config, not from the skill, so there is no path for a skill to mis-tag a quote. Table in "Price-signal role mapping" above. |
| 9.4 | Passed-goods candidates with quotes | N/A (no live runs) | Machinery in place: candidate selection joins through `qualification_runs` on `goods_qualified = 1 AND expiry_qualified = 1` so only passed-goods candidates are ever dispatched. |
| 9.5 | Raw margin estimate possible | N/A (no live runs) | Margin computation is Stage 10+, not Stage 9. |
| 9.6 | FK integrity | PASS (static) | `supplier_quotes.search_attempt_id` references the attempt row created in the same transaction; `product_candidate_id` references an existing `product_candidates.id` (validated by the join in `load_candidates_for_marketplace_search`). |
| 9.B1 | Skill schema validity | PASS | 10 JSON schema files parse as valid JSON. See "Data Snapshot". |
| 9.B2 | Candidate-selection query | PASS | Query returns exactly 23 eligible + 34 marginal = 57 candidates against the current `state.db`. |
| 9.B3 | `cargo check --all-targets` | PASS | Clean. |
| 9.B4 | `cargo clippy --all-targets -- -D warnings` | PASS | Clean (pedantic + nursery enabled). |
| 9.B5 | `cargo fmt --check` | PASS | Clean after one auto-format pass. |
| 9.B6 | `cargo build --release` | PASS | Daemon + desktop release binaries compile in ~1 min. |

## Code Changes

| File | Change |
|---|---|
| `skills/marketplace-search/gem-imports/`   | 5 new files (SKILL.md + 4 schemas) |
| `skills/marketplace-search/mx-wholesale/`  | 5 new files |
| `skills/marketplace-search/amazon-uk/`     | 5 new files |
| `skills/marketplace-search/ebay-uk/`       | 5 new files |
| `skills/marketplace-search/alibaba/`       | 5 new files |
| `crates/daemon/src/sources/registry.rs`    | +5 builtin definitions in `builtin_definitions()` (`gem-imports`, `mx-wholesale`, `amazon-uk`, `ebay-uk`, `alibaba-search`) with per-source `config` carrying `price_signal_role`, `region`, `quote_class` |
| `crates/daemon/src/main.rs` (constants)    | `DEFAULT_UK_RETAIL_WHOLESALE_DISCOUNT_BPS = 3000`; `MARKETPLACE_SEARCH_BATCH_SIZE = 5` |
| `crates/daemon/src/main.rs` (helpers)      | `is_marketplace_search_skill`, `marketplace_search_source_id`, `convert_to_gbp_pence` |
| `crates/daemon/src/main.rs` (persist)      | `persist_marketplace_search_results` — reads `result_json.search_results[]`, writes one `search_attempts` row per entry + one `supplier_quotes` row per quote; stage-tracks `price_signal_match` with `scope_kind = "candidate_source_pair"` |
| `crates/daemon/src/main.rs` (candidate load) | `load_candidates_for_marketplace_search` — JOINs `search_profiles` × `product_candidates` × `qualification_runs`, filters to eligible/marginal × passed-goods × recent-attempts gap, orders by ascending `recall_risk_score_bps`, batches at `MARKETPLACE_SEARCH_BATCH_SIZE` |
| `crates/daemon/src/main.rs` (live loop)    | `run_marketplace_search_live_loop(source_id)` — creates + claims `workflow_jobs` row with `kind = marketplace_search`, executes PTY skill session, reads `result.json`, calls `persist_marketplace_search_results`, finalises the job |
| `crates/daemon/src/main.rs` (CLI dispatch) | `--ingest gem-imports|mx-wholesale|amazon-uk|ebay-uk|alibaba-search` routes to the new live loop |
| `crates/daemon/src/main.rs` (`run_bound_skill`) | Added marketplace-search branch so `--skill-path skills/marketplace-search/<source>` with `--job-parameters-json` also persists via `persist_marketplace_search_results` |
| `crates/daemon/src/main.rs` (`source_lane_request`) | New Supplier source IDs join the arbiter-kind match; `marketplace-search/<suffix>` skill IDs resolve to the correct registry `source_id` (with `alibaba` → `alibaba-search` remap) |

## Issues Found & Fixed

During build:

1. **Clippy `doc_markdown`** — tabular doc comments mentioned `search_attempts`,
   `supplier_quotes`, `product_candidates` without backticks. Fixed by
   wrapping in backticks.
2. **Clippy `match_same_arms`** — `convert_to_gbp_pence` had both the `"GBP"`
   arm and the `_` wildcard returning the raw value. Removed the redundant
   `"GBP"` arm so the wildcard handles both cases (with a comment explaining
   it).
3. **`with_write_transaction` tuple shape** — I initially destructured the
   closure's `Result<(attempts, quotes)>` into a flat tuple, forgetting that
   `with_write_transaction` wraps the user value in
   `(user_value, commit_seq)`. Fixed the destructure to
   `((attempts, quotes), _commit_seq)`.
4. **Rustfmt** — four small formatting deltas after the large edits; `cargo
   fmt` auto-resolved them.

## Issues Remaining (Operational)

These are out-of-scope for a "build Stage 9" session but are what stands
between this manifest and GREEN status on checks 9.1–9.6:

1. **Live runs.** Run `target/release/daemon --ingest <source>` five times
   (once per source), or chain them in a small shell loop. Each invocation
   handles one batch of `MARKETPLACE_SEARCH_BATCH_SIZE = 5` candidates and
   can be run repeatedly; the `NOT EXISTS` clause in
   `load_candidates_for_marketplace_search` skips candidates already searched
   in the last 7 days.
2. **agent-browser connectivity.** Each skill assumes `agent-browser` can
   reach the target marketplace. Sites that deploy aggressive anti-bot
   measures (Amazon's "Are you a robot?", Alibaba's CAPTCHA challenge) will
   surface as `search_failure_mode: "source_captcha_blocked"`. Operator
   action: solve the interstitial manually or cool off per-source.
3. **Skill drift.** The playbooks are based on the current structure of each
   site. Expect one round of small refinements (selector changes, URL shape
   drift) after the first real run.
4. **USD→GBP FX.** `convert_to_gbp_pence` uses a fixed 79 pence ≈ 1 USD as
   documented in the task spec. A downstream reprice job against live FX
   is a separate concern.
5. **Search wave escalation.** The playbooks walk waves 1→4 and stop at the
   first wave producing a shortlisted quote. If a source is consistently
   returning zero on wave 1 but finding on wave 3+, revisit Stage 8's wave
   generation.

## How to run live (operational steps)

```bash
# Build once (already verified during manifest generation)
cargo build --release

# Dispatch a batch of 5 candidates against Gem Imports
target/release/daemon \
  --state-root state \
  --runtime-dir runtime \
  --artifact-root artifacts \
  --ingest gem-imports

# Repeat for the other four sources
target/release/daemon --ingest mx-wholesale   # UK wholesale #2
target/release/daemon --ingest amazon-uk      # UK retail ceiling
target/release/daemon --ingest ebay-uk        # UK retail ceiling
target/release/daemon --ingest alibaba-search # CN floor

# Keep looping until load_candidates_for_marketplace_search returns
# empty — at that point every passed-goods candidate has a recent
# attempt row for that source.
```

To scope-limit a first live run to a single candidate for faster feedback,
use the manual skill-path path instead:

```bash
target/release/daemon \
  --skill-path skills/marketplace-search/gem-imports \
  --job-parameters-json "$(cat skills/marketplace-search/gem-imports/canary.inputs.jsonl | head -1)"
```

## Data Snapshot

### Candidate query (verified 2026-04-19)
```
sqlite> SELECT COUNT(*), <eligibility split>  FROM ...
57|23|34    ← total / eligible / marginal
```

### Schema validity (10 files)
```
OK: skills/marketplace-search/alibaba/result.schema.json
OK: skills/marketplace-search/amazon-uk/result.schema.json
OK: skills/marketplace-search/ebay-uk/result.schema.json
OK: skills/marketplace-search/gem-imports/result.schema.json
OK: skills/marketplace-search/mx-wholesale/result.schema.json
OK: skills/marketplace-search/alibaba/checkpoint.schema.json
OK: skills/marketplace-search/amazon-uk/checkpoint.schema.json
OK: skills/marketplace-search/ebay-uk/checkpoint.schema.json
OK: skills/marketplace-search/gem-imports/checkpoint.schema.json
OK: skills/marketplace-search/mx-wholesale/checkpoint.schema.json
```

### Canary input validity (5 files)
```
OK × 5 (one canary candidate per source, each with 4-wave query plan)
```

### Compile status
```
cargo check --all-targets            → clean
cargo clippy --all-targets -- -D w.. → clean (pedantic + nursery enabled)
cargo fmt --check                    → clean
cargo build --release                → clean, 1m 06s wall time
```

### Pre-Stage-9 pipeline state (unchanged)
```
545 product_candidates in state.db
545 search_profiles (100 % coverage of scored candidates)
63 passed-goods candidates across 4 contracts with tender enrichment
57 searchable (eligible + marginal) = 23 + 34  (Stage 8 key metric)
6  ineligible
```

### Sample top-5 candidates the first Stage-9 batch will pick up
```
pc-7046e8ec-..  Double-glazed unit (sealed insulating glass unit, BS EN 1279…)
pc-169ba131-..  Print Services (CCP074 DPS umbrella)
pc-7e2a6fb2-..  Metal railings / gates / fencing — repair and modification
pc-7e2a6fb2-..  Chute / balcony balustrade — general repair under Lot 1 scope
pc-7e2a6fb2-..  Manufactured metal railings — supply and installation
```
Ordered by ascending `recall_risk_score_bps` per
`load_candidates_for_marketplace_search`; this matches the Stage 8 top-5
exactly.
