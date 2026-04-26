# Full Pipeline Re-Run: Ingest New Data → Run Through All Stages 1-9

ULTRATHINK and be meticulous.

## FRESH-EYES CURRENT-STATE OVERRIDE (2026-04-26)

This handoff was written before the full rerun landed. `docs/pipeline/full-pipeline-rerun-manifest.md` records the completed rerun and should be read before doing anything destructive or expensive.

Current read-only DB snapshot:

```text
contract_notices=133
procurement_processes=133
notice_lots=179
qualification_runs=308
product_candidates=820
search_profiles=820
search_attempts=193
supplier_quotes=10
candidate_pricing_assessments=4
landed_cost_estimates=0
```

Important correction: the ProContract interest-registration prefilter already uses a 14-day threshold in `candidate_deadline_check`; do **not** blindly apply the old "30 -> 14" change again. Treat this handoff as a rerun/playbook reference. A fresh agent's current mission should be to verify whether new notices/documents exist, run only the needed incremental steps, and update the manifest with a before/after delta.

## MISSION

Run an incremental full-pipeline refresh only where current evidence shows new data or stale stages. The previous rerun already moved the system to 133 notices, 820 product candidates/search profiles, 193 search attempts, 10 supplier quotes, and 4 gross-margin assessments; the goal now is not to replay old work blindly, but to ingest/enrich genuinely new ProContract data, drain new document extraction work, rerun deterministic stages that need it, and improve quote coverage where candidate selection returns useful work. Preserve a clear before/after manifest for every run.

Potential work items:

1. **Re-scrape ProContract** for new/updated opportunities (Stage 1)
2. **Re-run interest registration** on new contracts that meet criteria (Stage 2 — ProContract-specific)
3. **Check for tender responses** and enrich with full details + documents (Stage 3 — ProContract-specific)
4. **Verify the expiry threshold remains 14 days** — do not re-apply the old 30→14 change blindly
5. **Run the full deterministic pipeline** (Stages 4a, 6, 4b, 8, 3) on all new data
6. **Run document extraction** on any new tender documents (Stages 5b, 5c)
7. **Run marketplace search** on any new eligible candidates (Stage 9)

The goal: by the end, we have significantly more supplier quotes and dual-source pricing pairs to feed into Stage 10 (Gross Margin Gate).

```
┌──────────────────────────────────────────────────────────────────┐
│                    THE PIPELINE RE-RUN                            │
│                                                                  │
│   1. Verify expiry threshold is still 14 days                    │
│   2. Re-run ProContract 3-stage ingestion pipeline               │
│      a. --ingest procontract       (discover new opportunities)  │
│      b. --ingest procontract-interest (register interest)         │
│      c. --ingest procontract-enrich   (check for responses)      │
│   3. Run document extraction on new documents                    │
│      (--run-next-job until queue drains)                         │
│   4. Run deterministic pipeline (--pipeline procontract)          │
│      Stages: extract lots → prequalify → qualify lot →           │
│              reclassify → score quoteability → relate            │
│   5. Run marketplace search on new eligible candidates           │
│      (--ingest for each marketplace source)                      │
│   6. Validate the full pipeline output                           │
│   7. Write manifest                                              │
│                                                                  │
│   LOOP: fix issues at any stage, re-run, repeat until green      │
└──────────────────────────────────────────────────────────────────┘
```

---

## CONTEXT: Current State

```
Pipeline tables (before this re-run):
  contract_notices:        103  (all ProContract)
  procurement_processes:   103
  notice_lots:             141
  qualification_runs:      244  (103 notice-level + 141 lot-level)
  product_candidates:      545  (411 sku + 134 service)
  search_profiles:         545
  search_attempts:         76
  supplier_quotes:         5    (3 eBay + 2 Alibaba, all on 1 DGU candidate)
  document_extractions:    202
  semantic_extractions:    108

Key problem:
  All 10 goods-qualified contracts are DPSs → Stage 9 picker excludes them
  Need: non-DPS direct-order goods contracts
  
  Also: 4 contracts rejected as "expired" (30-day threshold too aggressive)
  Changing to 14 days unlocks ~10 additional contracts for qualification
```

---

## PHASE 1: Change Expiry Threshold (30 → 14 days)

### 1A. Find and change the threshold

There are TWO expiry checks in the codebase:

**Location 1 — `run_prequalify_stage()`** (the deterministic pipeline prequalification):
```bash
grep -n "num_days.*>=\|Duration::days(30)" crates/daemon/src/main.rs
```
Find the line with `remaining.num_days() >= 0` — this is the notice-level prequalify. It currently checks `deadline >= today` (any future deadline passes). This is actually fine — it doesn't use 30 days.

**Location 2 — ProContract interest registration pre-filter:**
```bash
grep -n "Duration::days(30)" crates/daemon/src/main.rs
```
Find the line: `deadline_utc > now + chrono::Duration::days(30)` — this is in the interest registration daemon logic that decides whether to register interest on a ProContract contract. **This is the one to change.**

### 1B. Make the change

Change `chrono::Duration::days(30)` to `chrono::Duration::days(14)`:

```rust
// Before:
deadline_utc > now + chrono::Duration::days(30),
// After:
deadline_utc > now + chrono::Duration::days(14),
```

### 1C. Rebuild

```bash
cargo check --all-targets && cargo clippy --all-targets -- -D warnings
cargo build --release --bin daemon
```

### 1D. Reset qualification for affected contracts

The 4 contracts that were previously rejected as "expired" (with the 30-day threshold) need to be re-qualified. Also, any contracts with deadlines in the 14-30 day window need fresh interest registration evaluation.

```bash
# Check which contracts would be affected
sqlite3 state/state.db "
SELECT cn.id, cn.title, cn.deadline_at,
  CAST(julianday(cn.deadline_at) - julianday('now') AS INTEGER) as days_remaining
FROM contract_notices cn
JOIN qualification_runs qr ON qr.notice_id = cn.id AND qr.qualification_basis = 'notice_only'
WHERE qr.expiry_qualified = 0
ORDER BY cn.deadline_at;
"
```

To re-qualify these, reset their prequalify + qualify-lot stage heads:
```bash
sqlite3 state/state.db "
-- Reset prequalify for expired contracts so they get re-evaluated
DELETE FROM process_stage_heads WHERE stage = 'prequalify notice' 
  AND process_id IN (
    SELECT qr.process_id FROM qualification_runs qr 
    WHERE qr.qualification_basis = 'notice_only' AND qr.expiry_qualified = 0
  );
DELETE FROM process_stage_status WHERE stage = 'prequalify notice'
  AND process_id IN (
    SELECT qr.process_id FROM qualification_runs qr 
    WHERE qr.qualification_basis = 'notice_only' AND qr.expiry_qualified = 0
  );
DELETE FROM qualification_runs WHERE expiry_qualified = 0;
"
```

---

## PHASE 2: Re-Run ProContract 3-Stage Ingestion

### 2A. Stage 1 — Discover new opportunities

```bash
# Run discovery — scrapes the next pages of ProContract opportunities
# Current cursor: page 3. This will continue from page 4+
cargo run --release --bin daemon -- --ingest procontract \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

Run repeatedly until the daemon reports "contracts-finder live loop completed" or similar (it processes one page per run with up to 100 notices):

```bash
while true; do
  OUTPUT=$(cargo run --release --bin daemon -- --ingest procontract \
    --state-root state --runtime-dir runtime --artifact-root artifacts 2>&1)
  echo "$OUTPUT" | tail -3
  if echo "$OUTPUT" | grep -q "continuation_required.*false\|completed.*notices=0"; then
    echo "=== Discovery complete ==="
    break
  fi
  if echo "$OUTPUT" | grep -q "Error:"; then
    echo "=== Discovery failed — diagnose and fix ==="
    break
  fi
done
```

Check what was found:
```bash
sqlite3 state/state.db "SELECT COUNT(*) as total_notices FROM contract_notices;"
```

### 2B. Stage 2 — Register interest on qualifying new contracts

This uses the NEW 14-day threshold. Only contracts with deadline > now + 14 days AND goods-likely will get interest registered.

```bash
cargo run --release --bin daemon -- --ingest procontract-interest \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

Check results:
```bash
sqlite3 state/state.db "
SELECT stage, state, COUNT(*) FROM process_stage_status 
WHERE stage = 'interest_registration' GROUP BY stage, state;
"
```

### 2C. Stage 3 — Check for tender responses and enrich

This checks the "Recently Added" tab on ProContract's home page for tenders that responded to our interest requests, downloads full details + documents.

```bash
cargo run --release --bin daemon -- --ingest procontract-enrich \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

Run repeatedly — the enrichment cursor advances through the "Recently Added" pages:
```bash
while true; do
  OUTPUT=$(cargo run --release --bin daemon -- --ingest procontract-enrich \
    --state-root state --runtime-dir runtime --artifact-root artifacts 2>&1)
  echo "$OUTPUT" | tail -3
  if echo "$OUTPUT" | grep -q "completed\|no candidates\|found 0"; then
    echo "=== Enrichment sweep complete ==="
    break
  fi
  if echo "$OUTPUT" | grep -q "Error:"; then
    echo "=== Enrichment failed — diagnose and fix ==="
    break
  fi
done
```

Check what was enriched:
```bash
sqlite3 state/state.db "
SELECT stage, state, COUNT(*) FROM process_stage_status WHERE stage = 'tender_enrichment' GROUP BY stage, state;
SELECT COUNT(*) as tender_doc_sets FROM tender_document_sets;
SELECT COUNT(*) as raw_documents_with_files FROM raw_documents WHERE storage_path IS NOT NULL;
"
```

---

## PHASE 3: Run Document Extraction on New Documents

Any new tender documents from the enrichment step need structural + semantic extraction.

```bash
# Check for pending extraction jobs
sqlite3 state/state.db "
SELECT kind, status, COUNT(*) FROM workflow_jobs 
WHERE kind LIKE 'extract%' AND status = 'pending'
GROUP BY kind, status;
"
```

If there are pending extraction jobs, run them:
```bash
COMPLETED=0; FAILED=0
while true; do
  OUTPUT=$(cargo run --release --bin daemon -- --run-next-job \
    --state-root state --runtime-dir runtime --artifact-root artifacts 2>&1)
  if echo "$OUTPUT" | grep -q "completed"; then
    COMPLETED=$((COMPLETED + 1))
  elif echo "$OUTPUT" | grep -q "no pending\|no claimable"; then
    echo "Queue drained: $COMPLETED completed, $FAILED failed"
    break
  else
    FAILED=$((FAILED + 1))
  fi
  if [ $((COMPLETED % 10)) -eq 0 ] && [ $COMPLETED -gt 0 ]; then
    echo "Progress: $COMPLETED completed, $FAILED failed"
  fi
done
```

Check extraction results:
```bash
sqlite3 state/state.db "
SELECT 'Structural extractions: ' || COUNT(*) FROM document_extractions;
SELECT 'Semantic extractions:   ' || COUNT(*) FROM document_semantic_extractions;
SELECT 'Product candidates:     ' || COUNT(*) FROM product_candidates;
"
```

---

## PHASE 4: Run Deterministic Pipeline

This runs all deterministic stages on the new data: extract lots → prequalify → qualify lot → reclassify candidates → score quoteability → relate.

```bash
cargo run --release --bin daemon -- --pipeline procontract \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

Check results:
```bash
sqlite3 state/state.db "
SELECT '=== QUALIFICATION FUNNEL (updated) ===';
SELECT 
  CASE
    WHEN qr.expiry_qualified = 0 THEN 'REJECTED: expired'
    WHEN qr.goods_qualified = 1 AND qr.goods_confidence_bps >= 6000 THEN 'PASSED: goods'
    WHEN qr.goods_qualified = 1 THEN 'REVIEW: low confidence goods'
    WHEN qr.legal_category = 'unknown' THEN 'REVIEW: unknown category'
    ELSE 'REJECTED: services/works'
  END as outcome,
  COUNT(*)
FROM qualification_runs qr WHERE qr.qualification_basis = 'notice_only'
GROUP BY outcome ORDER BY COUNT(*) DESC;
SELECT '';
SELECT '=== NON-DPS GOODS CONTRACTS ===';
SELECT cn.id, SUBSTR(cn.title, 1, 60) as title, qr.goods_qualified, qr.expiry_qualified
FROM contract_notices cn
JOIN qualification_runs qr ON qr.notice_id = cn.id AND qr.qualification_basis = 'notice_only'
WHERE qr.goods_qualified = 1 AND qr.expiry_qualified = 1
  AND cn.title NOT LIKE '%DPS%'
  AND cn.title NOT LIKE '%Dynamic Purchasing System%'
  AND cn.title NOT LIKE '%Acceptance Round%'
ORDER BY cn.title;
SELECT '';
SELECT '=== SEARCHABLE CANDIDATES (eligible + marginal) ===';
SELECT sp.search_eligibility_state, COUNT(*) FROM search_profiles sp
  JOIN product_candidates pc ON pc.id = sp.product_candidate_id
  JOIN qualification_runs qr ON qr.process_id = pc.process_id
  WHERE qr.qualification_basis = 'notice_only' AND qr.goods_qualified = 1 AND qr.expiry_qualified = 1
    AND sp.search_eligibility_state IN ('eligible', 'marginal')
    AND pc.candidate_type != 'service'
  GROUP BY sp.search_eligibility_state;
"
```

---

## PHASE 5: Run Marketplace Search on New Candidates

If the Stage 9 picker now finds non-DPS goods candidates, run marketplace search:

```bash
# Check if there are candidates to search
sqlite3 state/state.db "
SELECT COUNT(*) as stage9_candidates FROM search_profiles sp
  JOIN product_candidates pc ON pc.id = sp.product_candidate_id
  JOIN qualification_runs qr ON qr.process_id = pc.process_id
  JOIN contract_notices cn ON cn.process_id = pc.process_id
  WHERE qr.qualification_basis = 'notice_only' 
    AND qr.goods_qualified = 1 AND qr.expiry_qualified = 1
    AND sp.search_eligibility_state IN ('eligible', 'marginal')
    AND pc.candidate_type != 'service'
    AND cn.title NOT LIKE '%DPS%'
    AND cn.title NOT LIKE '%Dynamic Purchasing System%'
    AND cn.title NOT LIKE '%Acceptance Round%';
"
```

If > 0, run marketplace search for each source:
```bash
for SOURCE in gem-imports mx-wholesale alibaba-search amazon-uk ebay-uk; do
  echo "=== Running $SOURCE ==="
  while true; do
    OUTPUT=$(cargo run --release --bin daemon -- --ingest $SOURCE \
      --state-root state --runtime-dir runtime --artifact-root artifacts 2>&1)
    echo "$OUTPUT" | tail -2
    if echo "$OUTPUT" | grep -q "found no candidates\|Error:"; then break; fi
  done
done
```

**Note on Alibaba:** requires CDP Chrome profile. Before running alibaba-search:
```bash
# Start Chrome with debugging port (if not already running)
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
  --user-data-dir="$HOME/.chrome-automation" \
  --remote-debugging-port=9222 &
```

---

## PHASE 6: VALIDATION

### Check 1: More data ingested
```bash
sqlite3 state/state.db "
SELECT 'Total notices:           ' || COUNT(*) FROM contract_notices;
SELECT 'Total processes:         ' || COUNT(*) FROM procurement_processes;
SELECT 'Total lots:              ' || COUNT(*) FROM notice_lots;
SELECT 'Total product_candidates:' || COUNT(*) FROM product_candidates;
SELECT 'Total search_profiles:   ' || COUNT(*) FROM search_profiles;
SELECT 'Total search_attempts:   ' || COUNT(*) FROM search_attempts;
SELECT 'Total supplier_quotes:   ' || COUNT(*) FROM supplier_quotes;
"
```
**PASS:** Notices > 103 (new data ingested) OR at least more candidates have quotes.

### Check 2: Qualification funnel with 14-day threshold
```bash
sqlite3 state/state.db "
SELECT 'Expired (14-day threshold):' || COUNT(*) FROM qualification_runs 
WHERE qualification_basis = 'notice_only' AND expiry_qualified = 0;
"
```
**PASS:** Fewer expired contracts than with the 30-day threshold (was 4 before).

### Check 3: Non-DPS goods contracts available
```bash
sqlite3 state/state.db "
SELECT COUNT(*) as non_dps_goods FROM contract_notices cn
  JOIN qualification_runs qr ON qr.notice_id = cn.id AND qr.qualification_basis = 'notice_only'
  WHERE qr.goods_qualified = 1 AND qr.expiry_qualified = 1
    AND cn.title NOT LIKE '%DPS%' AND cn.title NOT LIKE '%Dynamic Purchasing System%'
    AND cn.title NOT LIKE '%Acceptance Round%';
"
```
**PASS:** > 0 non-DPS goods contracts exist.

### Check 4: Supplier quote coverage improved
```bash
sqlite3 state/state.db "
SELECT source_name, price_signal_role, COUNT(*) FROM supplier_quotes GROUP BY source_name, price_signal_role;
SELECT '';
SELECT 'Total quotes:  ' || COUNT(*) FROM supplier_quotes;
SELECT 'Quoted candidates: ' || COUNT(DISTINCT product_candidate_id) FROM supplier_quotes;
"
```
**PASS:** More quotes than the starting 5, or at least more candidates quoted.

### Check 5: Pipeline stage consistency
```bash
sqlite3 state/state.db "
SELECT stage, state, COUNT(*) FROM process_stage_status 
GROUP BY stage, state ORDER BY stage, state;
"
```
**PASS:** All stages have `succeeded` entries. No unexpected `failed` states.

### Check 6: FK integrity
```bash
sqlite3 state/state.db "
SELECT 'Orphan candidates (lot):    ' || COUNT(*) FROM product_candidates pc LEFT JOIN notice_lots nl ON nl.id = pc.lot_id WHERE nl.id IS NULL;
SELECT 'Orphan quotes (candidate):  ' || COUNT(*) FROM supplier_quotes sq LEFT JOIN product_candidates pc ON pc.id = sq.product_candidate_id WHERE pc.id IS NULL;
SELECT 'Orphan attempts (candidate):' || COUNT(*) FROM search_attempts sa LEFT JOIN product_candidates pc ON pc.id = sa.product_candidate_id WHERE pc.id IS NULL;
"
```
**PASS:** All 0.

---

## PHASE 7: FIX LOOP

If any phase fails:
1. Read the error message
2. Diagnose: code bug? data issue? site blocking? CAPTCHA?
3. Fix the root cause
4. Re-run from the failed phase (not from the beginning — earlier phases are idempotent)
5. Re-validate
6. Repeat until all checks pass

**Common issues and fixes:**

| Issue | Fix |
|---|---|
| ProContract login failed | Check credentials in SKILL.md: `sales@sorellon.com` / `9928fJG3!5nk6nc` |
| Interest registration CAPTCHA | Wait 5 min, retry |
| Enrichment finds no new responses | Expected — buyers take days/weeks to respond to interest requests |
| Document extraction fails (`.doc` format) | Already fixed — `textutil` conversion handles this |
| Semantic extraction `AGENT_PROTOCOL_MISMATCH` | Already fixed — contract-mode parser handles Claude stream-json |
| Prequalify shows all "unknown" | ProContract notices lack CPV codes — check if the `categories` field extraction is working (Phase 9 fix from Branch A) |
| Stage 9 picker returns 0 candidates | All qualified contracts are DPS — need more non-DPS ingestion |

---

## DELIVERABLES

1. **Expiry threshold changed** from 30 to 14 days
2. **ProContract pipeline re-run** complete (discover → interest → enrich)
3. **Document extraction** run on any new documents
4. **Deterministic pipeline** run on all new data
5. **Marketplace search** run on any new eligible candidates
6. **All validation checks pass**
7. **Compile clean** if any code changes

### MANDATORY: Write Results Manifest

After all phases complete, write to `docs/pipeline/full-pipeline-rerun-manifest.md`:

```markdown
# Full Pipeline Re-Run Manifest

> Generated: {timestamp}
> Status: {GREEN / YELLOW / RED}

## Changes Made
- Expiry threshold: 30 days → 14 days

## Data Delta (before → after)

| Table | Before | After | Delta |
|-------|--------|-------|-------|
| contract_notices | 103 | X | +X |
| procurement_processes | 103 | X | +X |
| notice_lots | 141 | X | +X |
| qualification_runs | 244 | X | +X |
| product_candidates | 545 | X | +X |
| search_profiles | 545 | X | +X |
| search_attempts | 76 | X | +X |
| supplier_quotes | 5 | X | +X |
| document_extractions | 202 | X | +X |
| semantic_extractions | 108 | X | +X |

## Qualification Funnel (after re-run)

| Outcome | Count |
|---------|-------|
| PASSED: goods | X |
| REJECTED: expired | X |
| REJECTED: services/works | X |
| REVIEW: unknown | X |

## Non-DPS Goods Contracts

| Title | Has Candidates | Has Quotes |
|-------|---------------|------------|
| ... | YES/NO | YES/NO |

## Key Metrics
- **Non-DPS goods contracts: X**
- **Searchable candidates (non-DPS, non-service): X**
- **Total supplier quotes: X (was 5)**
- **Candidates with dual-source pricing: X (was 1)**

## Issues Found & Fixed
{...}

## Data Snapshot
{Full validation query output}
```

---

## CONSTRAINTS

1. **Work in a loop.** Fix issues at any phase, re-run, repeat.
2. **Phases are sequential** but each phase is idempotent — re-running skips already-completed work.
3. **Don't skip the ProContract 3-stage pipeline.** The order matters: discover → interest → enrich.
4. **Document extraction is slow.** Structural extraction is fast (Docling), semantic extraction is slow (Claude). Budget time accordingly.
5. **Marketplace search requires CDP Chrome for Alibaba.** Start it before running alibaba-search.
6. **Rust conventions** for any code changes: `cargo check --all-targets && cargo clippy --all-targets -- -D warnings`.

---

## REMINDERS

- ULTRATHINK and be meticulous.
- **The whole point is MORE DATA.** The pipeline works correctly — it's just starved of non-DPS goods contracts. This re-run is about feeding it more raw material.
- The 14-day threshold change is a single line of code. Don't overthink it.
- ProContract discovery continues from where it left off (cursor at page 3). It will scrape page 4+ for new opportunities.
- Enrichment depends on buyers responding to interest requests — if no new responses exist, the enrichment step will correctly report 0 enriched. That's not a bug.
- Each marketplace search run processes up to 3 candidates (`MARKETPLACE_SEARCH_BATCH_SIZE = 3`). Run repeatedly until the picker returns empty.
