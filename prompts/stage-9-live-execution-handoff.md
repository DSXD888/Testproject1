# Stage 9 Live Execution: Run Marketplace Search Skills Against Real Sites

ULTRATHINK and be meticulous.

## FRESH-EYES CURRENT-STATE OVERRIDE (2026-04-26)

This handoff was originally written for the first live Stage 9 execution pass. The project has moved since then: current DB has `search_attempts=193` and `supplier_quotes=10`, with quotes from `alibaba-search`, `amazon-uk`, and `ebay-uk`. `MARKETPLACE_SEARCH_BATCH_SIZE` is currently `3` in `crates/daemon/src/main.rs`, not 5.

Current source attempt distribution:

```text
alibaba-search=5
amazon-uk=35
ebay-uk=46
gem-imports=56
mx-wholesale=51
```

Use this handoff as a live-run repair guide, not a first-run checklist. A receiving agent should first inspect `docs/pipeline/stage-9-live-execution-manifest.md` and current DB state, then only run additional marketplace searches where candidate selection returns eligible work and cost/time makes sense.

## MISSION

Continue executing and validating the five marketplace search skills against real websites, using current DB state and the latest Stage 9 manifest as the starting point. The pipeline already has real supplier quotes; your job is to run only the remaining useful marketplace work, validate the pricing data that flows back, fix site-specific skill/daemon issues that arise, and stop when additional runs are no longer producing actionable quote coverage.

The Stage 9 **infrastructure** is built and compiled (YELLOW status from the build manifest). This handoff turns it GREEN by running the live searches and resolving the inevitable site-specific issues that only surface during real execution.

**You must work in an iterative loop.** For each source: run → check output → diagnose failures → fix the skill playbook or daemon code → re-run. Repeat until that source produces real quotes. Then move to the next source.

```
┌─────────────────────────────────────────────────────────────────┐
│               THE LOOP (per source)                              │
│                                                                  │
│   1. Run --ingest {source_id}                                    │
│   2. Check: did the daemon complete successfully?                │
│      ├─ NO (crash/error) → Read error, fix code, rebuild, retry │
│      └─ YES → continue                                          │
│   3. Check: did the skill find any quotes?                       │
│      ├─ NO (0 quotes) → Read PTY capture, diagnose why:          │
│      │   • Skill couldn't find the search bar?                   │
│      │   • Site blocked the agent (CAPTCHA)?                     │
│      │   • Search returned results but none matched?             │
│      │   • Skill wrote result.json but it was malformed?         │
│      │   → Fix SKILL.md playbook, retry                          │
│      └─ YES → validate data quality                             │
│   4. Validate: are prices reasonable? FK integrity?              │
│      ├─ ISSUES → fix and retry                                   │
│      └─ CLEAN → source is GREEN, move to next                   │
│                                                                  │
│   REPEAT FOR ALL 5 SOURCES                                       │
│   DO NOT STOP UNTIL ALL SOURCES HAVE BEEN ATTEMPTED              │
└─────────────────────────────────────────────────────────────────┘
```

---

## CONTEXT

**What's already built (from build manifest):**
- 5 skill packages under `skills/marketplace-search/` (25 files)
- 5 source registry entries in daemon
- `--ingest` match arms for all 5 sources
- `persist_marketplace_search_results()` persist function
- `load_candidates_for_marketplace_search()` candidate selection
- `run_marketplace_search_live_loop()` execution function
- `MARKETPLACE_SEARCH_BATCH_SIZE = 3` as of 2026-04-26 (query `crates/daemon/src/main.rs` before running; this value has changed before)
- All compiles clean (cargo check + clippy + release build)

**What needs to happen now:**
- Run `--ingest {source}` for each source against live websites
- Each run spawns a Claude CLI PTY session that uses agent-browser to navigate the marketplace
- The agent searches for products, extracts prices, writes result.json
- The daemon persists search_attempts and supplier_quotes to SQLite
- Fix any issues that arise (site structure changes, CAPTCHAs, skill playbook bugs)

**The 5 sources (run in this order):**

| # | Source ID | URL | Marketplace Type |
|---|-----------|-----|-----------------|
| 1 | `gem-imports` | `https://www.gemimports.co.uk/` | UK wholesale |
| 2 | `mx-wholesale` | `https://www.mxwholesale.co.uk/` | UK wholesale |
| 3 | `alibaba-search` | `https://www.alibaba.com/` | Chinese B2B |
| 4 | `amazon-uk` | `https://www.amazon.co.uk/` | UK retail |
| 5 | `ebay-uk` | `https://www.ebay.co.uk/` | UK retail |

---

## PREREQUISITES

### Verify everything compiles

```bash
cd /Users/alinaaitova/Desktop/Testproject1
cargo build --release --bin daemon 2>&1 | tail -3
```

### Verify agent-browser works

```bash
agent-browser --version
agent-browser open "https://www.google.com"
agent-browser wait
agent-browser close
```

### Read AGENTS.md — single source of truth for agent instructions.

### Verify candidate selection returns data

```bash
cargo run --release --bin daemon -- --ingest gem-imports --dry-run \
  --state-root state --runtime-dir runtime --artifact-root artifacts 2>&1 | tail -5
```

If `--dry-run` isn't supported for this path, run the full command and check output. The daemon should log how many candidates it selected.

---

## EXECUTION: Source by Source

### Source 1: Gem Imports (`gem-imports`)

**Run:**
```bash
cargo run --release --bin daemon -- --ingest gem-imports \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

This will:
1. Query the DB for up to 5 candidates not yet searched on gem-imports
2. Build job_parameters with candidate names, head nouns, query waves
3. Spawn a Claude CLI PTY session running `skills/marketplace-search/gem-imports/SKILL.md`
4. The agent uses agent-browser to navigate gemimports.co.uk, search for products, extract prices
5. Agent writes result.json to workdir
6. Daemon reads result.json, persists to search_attempts + supplier_quotes
7. Reports completion

**If it succeeds,** check results:
```bash
sqlite3 state/state.db "
SELECT 'gem-imports attempts: ' || COUNT(*) FROM search_attempts WHERE source = 'gem-imports';
SELECT 'gem-imports quotes:   ' || COUNT(*) FROM supplier_quotes WHERE source_name = 'gem-imports';
SELECT source_name, product_name, unit_price_gbp_pence, currency, price_signal_role
FROM supplier_quotes WHERE source_name = 'gem-imports' LIMIT 5;
"
```

**If it fails,** diagnose:

```bash
# Check the workflow job status
sqlite3 state/state.db "SELECT id, status, last_error FROM workflow_jobs WHERE kind = 'marketplace_search' ORDER BY created_at DESC LIMIT 1;"

# Find the session
sqlite3 state/state.db "SELECT id, terminal_output_path, exit_code, error_type, error_message FROM agent_session_logs ORDER BY created_at DESC LIMIT 1;"

# Read PTY capture (if it exists)
LATEST=$(ls -t artifacts/sessions/2026/04/ | head -1)
ls artifacts/sessions/2026/04/$LATEST/
zstd -dc artifacts/sessions/2026/04/$LATEST/stream/000001.pty.zst 2>/dev/null | strings | tail -30

# Check if result.json was written
ls artifacts/sessions/2026/04/$LATEST/workdir/result.json 2>/dev/null
cat artifacts/sessions/2026/04/$LATEST/workdir/result.json 2>/dev/null | python3 -m json.tool | head -20
```

**Common failure patterns and fixes:**

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| `AGENT_RUN_FAILED: did not produce result contract` | Skill crashed, no result.json written | Check PTY capture for errors. Fix SKILL.md playbook. |
| `parsing marketplace-search result JSON` | result.json exists but isn't valid JSON | Check result.json content. Usually the agent wrote narrative text instead of JSON. Add "CRITICAL: write result.json" to SKILL.md. |
| 0 quotes but attempts > 0 | Search worked but nothing matched | Check what the agent searched for vs what the site has. Adjust query waves or broaden matching criteria in SKILL.md. |
| `Error: locking runtime/daemon.lock` | Another daemon instance is running | Kill the other process: `pkill -f "target/release/daemon"` |
| CAPTCHA / "Are you a robot?" | Site detected automated browsing | Wait 5-10 minutes, retry. If persistent, add delays between requests in SKILL.md. |
| Agent-browser download errors | Browser connectivity issue | Run `agent-browser close --all` and retry. |

**After fixing, reset and re-run:**
```bash
# Reset failed job so load_candidates picks the candidates again
sqlite3 state/state.db "
UPDATE workflow_jobs SET status = 'failed' WHERE kind = 'marketplace_search' AND status = 'completed'
  AND id = (SELECT id FROM workflow_jobs WHERE kind = 'marketplace_search' ORDER BY created_at DESC LIMIT 1);
"
# Re-run
cargo run --release --bin daemon -- --ingest gem-imports \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

**Run repeatedly** until `load_candidates_for_marketplace_search` returns empty (all candidates have recent search attempts for this source):
```bash
# Keep running until "found no candidates needing search"
while true; do
  OUTPUT=$(cargo run --release --bin daemon -- --ingest gem-imports \
    --state-root state --runtime-dir runtime --artifact-root artifacts 2>&1)
  echo "$OUTPUT" | tail -3
  if echo "$OUTPUT" | grep -q "found no candidates needing search"; then
    echo "=== gem-imports COMPLETE ==="
    break
  fi
  if echo "$OUTPUT" | grep -q "Error:"; then
    echo "=== gem-imports FAILED — diagnose and fix ==="
    break
  fi
done
```

### Source 2: MX Wholesale (`mx-wholesale`)

Same process as Source 1, replacing `gem-imports` with `mx-wholesale`.

### Source 3: Alibaba (`alibaba-search`)

Same process, replacing with `alibaba-search`. **Special considerations:**
- Alibaba may show CAPTCHAs more aggressively — if blocked, wait and retry
- Prices are in USD — the daemon's `convert_to_gbp_pence` converts at 79 pence/USD
- Check that `currency = 'USD'` in the quotes, NOT 'GBP'
- Verify `price_tiers` data is captured in the `evidence` field

### Source 4: Amazon UK (`amazon-uk`)

Same process, replacing with `amazon-uk`. **Special considerations:**
- Amazon is the noisiest source — many irrelevant results
- Check that the agent is finding multi-pack/bulk options, not just single-unit prices
- "Are you a robot?" challenges are common — retry with delays

### Source 5: eBay UK (`ebay-uk`)

Same process, replacing with `ebay-uk`. **Special considerations:**
- The agent should filter for "Buy It Now" (not auctions) and "New" condition
- Check that `is_used_or_refurbished = 0` on all quotes

---

## VALIDATION (after all 5 sources have been run)

### Check 1: Search attempts across all sources
```bash
sqlite3 state/state.db "
SELECT source, COUNT(*) as attempts, SUM(shortlisted_count) as shortlisted
FROM search_attempts GROUP BY source ORDER BY attempts DESC;
SELECT '';
SELECT 'Total search_attempts: ' || COUNT(*) FROM search_attempts;
"
```
**PASS:** At least 3 sources have attempts. Total > 50.

### Check 2: Supplier quotes across all sources
```bash
sqlite3 state/state.db "
SELECT source_name, price_signal_role, COUNT(*) as quotes, 
       CAST(AVG(unit_price_gbp_pence) AS INTEGER) as avg_price_pence,
       MIN(unit_price_gbp_pence) as min_price, MAX(unit_price_gbp_pence) as max_price
FROM supplier_quotes GROUP BY source_name, price_signal_role ORDER BY source_name;
"
```
**PASS:** At least 2 sources have quotes. Alibaba/Chinese prices should be materially lower than UK prices for the same products.

### Check 3: Price signal roles present
```bash
sqlite3 state/state.db "
SELECT price_signal_role, COUNT(*) FROM supplier_quotes GROUP BY price_signal_role;
"
```
**PASS:** At least 2 of the 3 roles present (`uk_wholesale`, `uk_retail_benchmark`, `cn_supplier_floor`). Ideally all 3.

### Check 4: Candidate coverage
```bash
sqlite3 state/state.db "
SELECT 'Candidates with >=1 quote: ' || COUNT(DISTINCT product_candidate_id) FROM supplier_quotes sq
  JOIN product_candidates pc ON pc.id = sq.product_candidate_id
  JOIN qualification_runs qr ON qr.process_id = pc.process_id
  WHERE qr.qualification_basis = 'notice_only' AND qr.goods_qualified = 1 AND qr.expiry_qualified = 1;
SELECT 'Total searchable candidates: 57';
"
```
**PASS:** At least 5 of 57 candidates have quotes (not every product will be found on every source).

### Check 5: Dual-source pricing (UK + CN)
```bash
sqlite3 state/state.db "
SELECT pc.name,
  MIN(CASE WHEN sq.price_signal_role IN ('uk_wholesale', 'uk_retail_benchmark') THEN sq.unit_price_gbp_pence END) as uk_best_pence,
  MIN(CASE WHEN sq.price_signal_role = 'cn_supplier_floor' THEN sq.unit_price_gbp_pence END) as cn_best_pence,
  CASE WHEN MIN(CASE WHEN sq.price_signal_role IN ('uk_wholesale', 'uk_retail_benchmark') THEN sq.unit_price_gbp_pence END) IS NOT NULL
    AND MIN(CASE WHEN sq.price_signal_role = 'cn_supplier_floor' THEN sq.unit_price_gbp_pence END) IS NOT NULL
  THEN 'YES' ELSE 'no' END as dual_source
FROM supplier_quotes sq
JOIN product_candidates pc ON pc.id = sq.product_candidate_id
GROUP BY pc.id
ORDER BY dual_source DESC, uk_best_pence DESC
LIMIT 15;
"
```
**PASS:** At least 1 candidate has both UK and CN prices. This enables raw margin estimation.

### Check 6: FK integrity
```bash
sqlite3 state/state.db "
SELECT 'Orphan quotes (candidate):  ' || COUNT(*) FROM supplier_quotes sq
  LEFT JOIN product_candidates pc ON pc.id = sq.product_candidate_id WHERE pc.id IS NULL;
SELECT 'Orphan attempts (candidate): ' || COUNT(*) FROM search_attempts sa
  LEFT JOIN product_candidates pc ON pc.id = sa.product_candidate_id WHERE pc.id IS NULL;
SELECT 'Orphan attempts (profile):   ' || COUNT(*) FROM search_attempts sa
  LEFT JOIN search_profiles sp ON sp.id = sa.search_profile_id WHERE sp.id IS NULL;
"
```
**PASS:** All 0.

### Check 7: Data quality spot-check
```bash
sqlite3 state/state.db "
SELECT source_name, product_name, unit_price_gbp_pence, currency, price_signal_role,
       match_confidence_bps, SUBSTR(evidence, 1, 80) as evidence_preview
FROM supplier_quotes ORDER BY created_at DESC LIMIT 10;
"
```
**PASS:** Prices are reasonable (not 0, not absurdly high), currencies are correct (GBP for UK, USD for Alibaba), evidence text explains the match.

---

## IMPORTANT: Handling Partial Success

Not every source will work perfectly on the first try. Some sites may block automated browsing, some products may not exist on certain marketplaces. This is expected.

**The minimum viable outcome for Stage 9 GREEN is:**
- At least 3 of 5 sources produce quotes
- At least 5 candidates have at least 1 quote
- At least 1 candidate has both UK and CN pricing (enabling margin estimation)
- FK integrity clean

**If a source is persistently blocked** (e.g., aggressive CAPTCHA on every attempt):
- Document it as a known limitation in the manifest
- Move on to the next source
- Mark that source as YELLOW (not RED — the infrastructure works, the site is blocking)

---

## DELIVERABLES

1. **Live runs attempted** for all 5 sources
2. **All validation checks pass** (or documented partial-pass with explanation)
3. **Any SKILL.md fixes** made during the loop (document what changed)
4. **Any daemon code fixes** (with cargo check + clippy clean)

### MANDATORY: Write Results Manifest

After all sources have been attempted, write to `docs/pipeline/stage-9-live-execution-manifest.md`:

```markdown
# Stage 9 Live Execution Manifest

> Generated: {timestamp}
> Status: {GREEN / YELLOW / RED}

## Source Execution Summary

| Source | Runs | Candidates Searched | Attempts | Quotes | Status |
|--------|------|-------------------|----------|--------|--------|
| gem-imports | X | X | X | X | GREEN/YELLOW/RED |
| mx-wholesale | X | X | X | X | GREEN/YELLOW/RED |
| alibaba-search | X | X | X | X | GREEN/YELLOW/RED |
| amazon-uk | X | X | X | X | GREEN/YELLOW/RED |
| ebay-uk | X | X | X | X | GREEN/YELLOW/RED |

## Quote Distribution

| Price Signal Role | Count | Avg Price (pence) | Min | Max |
|------------------|-------|-------------------|-----|-----|
| uk_wholesale | X | X | X | X |
| uk_retail_benchmark | X | X | X | X |
| cn_supplier_floor | X | X | X | X |

## Dual-Source Pricing (Candidates with UK + CN quotes)

| Candidate | UK Best (pence) | CN Best (pence) | Raw Margin % |
|-----------|----------------|----------------|-------------|
| ... | ... | ... | ... |

## Key Metrics
- **Sources producing quotes: X of 5**
- **Candidates with >=1 quote: X of 57 (X%)**
- **Candidates with dual-source pricing: X**
- **Average raw margin (where dual-source): X%**

## Validation Checks
| # | Check | Result | Detail |
|---|-------|--------|--------|
| 1 | Attempts across sources | PASS/FAIL | ... |
| 2 | Quotes across sources | PASS/FAIL | ... |
| 3 | Price signal roles | PASS/FAIL | ... |
| 4 | Candidate coverage | PASS/FAIL | ... |
| 5 | Dual-source pricing | PASS/FAIL | ... |
| 6 | FK integrity | PASS/FAIL | ... |
| 7 | Data quality | PASS/FAIL | ... |

## SKILL.md Changes Made During Execution

{For each source: what playbook changes were needed, why}

## Issues Found & Fixed

{Daemon code fixes, if any}

## Issues Remaining

{Sites that are blocked, persistent problems, known limitations}

## Data Snapshot

{Full output of all validation queries}
```

---

## CONSTRAINTS

1. **Work source by source.** Don't try to run all 5 simultaneously — diagnose and fix each before moving on.
2. **Fix the SKILL.md playbook, not the website.** If the agent can't find a search bar, the playbook needs better instructions, not a code change.
3. **Each run processes up to 5 candidates** (`MARKETPLACE_SEARCH_BATCH_SIZE`). Run repeatedly until the daemon reports "found no candidates needing search."
4. **Don't fake data.** Every supplier_quote must come from a real marketplace listing. If a product isn't found, that's a legitimate zero-result search_attempt.
5. **Rust conventions** if any code changes: `cargo check --all-targets && cargo clippy --all-targets -- -D warnings`.
6. **Store RAW prices.** Never apply the wholesale discount — that's a Stage 10+ concern.

---

## REMINDERS

- ULTRATHINK and be meticulous.
- **The loop is non-negotiable.** Run → check → fix → repeat for each source.
- **PTY sessions take time.** Each search session is 2-10 minutes of Claude navigating a website. 57 candidates × 5 sources = potentially 285 sessions. Start with 1 batch of 5 candidates per source to validate, then expand.
- **Read the PTY capture when things fail.** `zstd -dc artifacts/sessions/.../stream/000001.pty.zst | strings | tail -50` shows what the agent actually did.
- **The most common failure is the agent not writing result.json.** If this happens, add explicit "CRITICAL: write result.json to the working directory" instructions to the SKILL.md and ensure the skill tells the agent to use the Write tool or cat heredoc.
- **Some products won't be found on some sources.** Double-glazed units won't be on Alibaba. Print services won't be on eBay. That's fine — record the zero-result search_attempt and move on.
- **Alibaba is the highest-value source** — it provides the Chinese floor price for margin calculation. Prioritize getting it working.
