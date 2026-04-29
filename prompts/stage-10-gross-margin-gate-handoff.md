# Build Stage 10: Gross Margin Gate

ULTRATHINK and be meticulous.

## FRESH-EYES CURRENT-STATE OVERRIDE (2026-04-26)

This handoff was originally written as a build prompt. **Stage 10 has now landed.** Current `crates/daemon/src/main.rs` contains `run_gross_margin_gate_stage()` and `run_deterministic_pipeline()` invokes it after `run_score_quoteability_stage()`.

Current read-only DB snapshot:

```text
supplier_quotes=10
candidate_pricing_assessments=4
process_stage_heads gross margin gate=4
```

Use this handoff as the design/validation source for Stage 10, but a fresh agent should not add a second gross-margin implementation. The current mission is **regression validation and targeted repair**: compare code and DB behavior to `docs/pipeline/stage-10-validation-manifest.md`, fix only real drift, and keep the Stage 10 -> Stage 11 contract stable.

## MISSION

Validate and maintain **Stage 10: Gross Margin Gate** as a real deterministic pipeline stage. The stage already exists in current code; it takes Stage 9 supplier quotes and produces per-candidate `candidate_pricing_assessments` with pass/review/reject economics. A receiving agent should verify the existing implementation against this handoff and `docs/pipeline/stage-10-validation-manifest.md`, then fix only genuine drift or regressions.

The stage is **pure deterministic Rust code** — no LLM, no agent skill, no browser. It reads `supplier_quotes` + `product_candidates` + contract values, applies the wholesale discount for retail benchmarks, computes margins, and writes assessments.

```
┌─────────────────────────────────────────────────────────┐
│                    THE LOOP                               │
│                                                          │
│   1. Build run_gross_margin_gate_stage()                 │
│   2. Wire into run_deterministic_pipeline()              │
│   3. Run --pipeline procontract                          │
│   4. Validate the output (10 checks below)               │
│   5. Any failures?                                       │
│      ├─ YES → Diagnose, fix, re-run, re-validate        │
│      └─ NO  → Stage 10 GREEN. Write manifest.            │
│                                                          │
│   DO NOT STOP UNTIL STEP 5 = NO                          │
└─────────────────────────────────────────────────────────┘
```

---

## CONTEXT: What Feeds Stage 10

```
From Stages 1-9 (GREEN):
  133 contract_notices           (with deadline, value where available)
  820 product_candidates         (411 sku + 134 service)
  820 search_profiles            (scored, eligibility classified)
  193 search_attempts            (across 5 marketplace sources)
  10  supplier_quotes            (5 eBay + 3 Amazon + 2 Alibaba)
  4   candidates with quotes:
      • Double-glazed unit (5 quotes — 1 dual-source UK+CN pair)
      • Aggregates (3 quotes)
      • Library carpet × 2 (2 quotes)

Stage 10 reads:
  supplier_quotes               → unit prices, price_signal_role, MOQ, evidence
  product_candidates            → candidate_value_share_bps (if set)
  contract_notices              → estimated_value_original_minor (revenue source)
  notice_lots                   → estimated_value_pence (lot-level value)

Stage 10 writes:
  candidate_pricing_assessments → one row per candidate with at least one quote
  process_stage_status          → 'gross margin gate' / 'candidate' scope
  process_stage_heads           → idempotency

Stage 10 also reads:
  DEFAULT_UK_RETAIL_WHOLESALE_DISCOUNT_BPS = 3000
    (the constant added in Stage 9 — applied to uk_retail_benchmark quotes)
```

---

## PHASE 0: CODEBASE RESEARCH (mandatory before coding)

### 0A. Read AGENTS.md — single source of truth.

### 0B. Stage Function Skeleton

Find and study `run_prequalify_stage()` and `run_score_quoteability_stage()` in `crates/daemon/src/main.rs`:

```bash
grep -n "fn run_prequalify_stage\|fn run_score_quoteability_stage" crates/daemon/src/main.rs
```

Every deterministic stage follows this skeleton:

```rust
fn run_STAGE_stage(state_db: &mut StateDb) -> Result<usize> {
    // 1. QUERY: find unprocessed entities
    let rows: Vec<(...)> = {
        let conn = state_db.connection();
        let mut stmt = conn.prepare("SELECT ... WHERE NOT EXISTS (... process_stage_heads ...)")?;
        stmt.query_map(...)?.collect::<Result<Vec<_>, _>>()?
    };
    if rows.is_empty() { return Ok(0); }

    // 2. COMPUTE + WRITE in a single transaction
    let stage_run_id = allocate_stage_run_id();
    let (count, _) = state_db.with_write_transaction(|tx| {
        let visible_after_seq = next_commit_seq(tx)?;
        let mut processed = 0_usize;
        for row in &rows {
            // ... compute ...
            // ... INSERT ... ON CONFLICT DO UPDATE ...
            // ... persist_stage(tx, process_id, "Stage name", scope_kind, scope_id, &stage_run_id) ...
            processed += 1;
        }
        Ok(processed)
    })?;
    Ok(count)
}
```

### 0C. The Output Table (`candidate_pricing_assessments`)

```sql
CREATE TABLE candidate_pricing_assessments (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  notice_id TEXT NOT NULL REFERENCES contract_notices(id),
  product_candidate_id TEXT NOT NULL REFERENCES product_candidates(id),
  selected_quote_id TEXT NOT NULL REFERENCES supplier_quotes(id),
  selected_quote_row_version INTEGER NOT NULL,
  quote_cost_boundary TEXT NOT NULL,           -- 'unit' / 'pack' / 'project'
  revenue_basis_type TEXT NOT NULL,            -- 'expected_value' / 'low_value' / 'high_value' / 'rfq_required'
  expected_addressable_revenue_ex_vat_pence INTEGER NOT NULL,
  revenue_confidence_bps INTEGER NOT NULL,     -- 0-10000
  gross_margin_bps INTEGER NOT NULL,           -- (revenue - cost) / revenue * 10000; can be negative
  gross_estimate_confidence_bps INTEGER NOT NULL,
  identity_gate_passed INTEGER NOT NULL,       -- 1 / 0
  usability_gate_passed INTEGER NOT NULL,      -- 1 / 0
  pricing_input_confidence_bps INTEGER NOT NULL,
  owner_id TEXT NOT NULL, row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER, stage_run_id TEXT
);
```

### 0D. Existing Fixture Pattern (reference only — do NOT use these values)

Search `INSERT INTO candidate_pricing_assessments` in main.rs to see the fixture-materialization pattern. The fixture uses hardcoded values (`expected_addressable_revenue_ex_vat_pence = 120000`, `gross_estimate_confidence_bps = 7600`, etc.). **Do NOT use these constants** — your stage computes them from real data.

### 0E. Source Schemas to Read From

```sql
-- supplier_quotes (Stage 9 output)
unit_price_gbp_pence INTEGER NOT NULL    -- raw price (pence for GBP, converted from USD/EUR)
price_signal_role TEXT NOT NULL          -- 'uk_wholesale' / 'uk_retail_benchmark' / 'cn_supplier_floor'
match_confidence_bps INTEGER             -- 0-10000
quote_usability_confidence_bps INTEGER
quote_realism_confidence_bps INTEGER
minimum_order_quantity NUMERIC
price_basis_qty NUMERIC
includes_shipping INTEGER NOT NULL DEFAULT 0
vat_inclusive INTEGER

-- product_candidates
quantity NUMERIC                         -- often NULL
candidate_value_share_bps INTEGER        -- often NULL — fall back to equal split
quoteability_score_bps INTEGER

-- contract_notices
estimated_value_original_minor INTEGER   -- contract value (pence for GBP)
estimated_value_currency TEXT            -- 'GBP' typically

-- notice_lots
estimated_value_pence INTEGER            -- lot value if known
```

### 0F. The Wholesale Discount

`DEFAULT_UK_RETAIL_WHOLESALE_DISCOUNT_BPS = 3000` is in `crates/daemon/src/main.rs`. Apply it to `uk_retail_benchmark` quotes ONLY:

```rust
let effective_cost_pence = match quote.price_signal_role.as_str() {
    "uk_retail_benchmark" => {
        // Estimate wholesale: retail × (10000 - 3000) / 10000 = retail × 0.70
        quote.unit_price_gbp_pence * (10_000 - DEFAULT_UK_RETAIL_WHOLESALE_DISCOUNT_BPS) / 10_000
    }
    _ => quote.unit_price_gbp_pence,
};
```

You must be able to answer before coding:
1. What does `persist_stage()` write?
2. Why apply the wholesale discount only to `uk_retail_benchmark` quotes?
3. What happens if a candidate has no quotes? (skip — no assessment possible)
4. What happens if the contract has no estimated value? (`revenue_basis_type = 'rfq_required'`, low confidence)

---

## PHASE 1: REVENUE ALLOCATION

For each quoted candidate, estimate the revenue addressable by that candidate.

### Allocation rules (in order of preference):

1. **Lot value × candidate share** (best):
   ```
   revenue = notice_lots.estimated_value_pence × (candidate_value_share_bps / 10000)
   confidence = 8500 bps
   ```

2. **Lot value, equal split across candidates** (when `candidate_value_share_bps` is NULL):
   ```
   revenue = notice_lots.estimated_value_pence / count_of_candidates_in_lot
   confidence = 6500 bps
   ```

3. **Contract value, equal split across all candidates in process** (when lot value is NULL):
   ```
   revenue = contract_notices.estimated_value_original_minor / count_of_candidates_in_process
   confidence = 4500 bps
   ```

4. **No value information available** (RFQ-required):
   ```
   revenue = 0
   confidence = 1000 bps
   revenue_basis_type = 'rfq_required'
   ```

`revenue_basis_type` should be `'expected_value'` for cases 1-3, `'rfq_required'` for case 4.

---

## PHASE 2: QUOTE SELECTION (Lexicographic Gating)

For each candidate with multiple quotes, select the BEST quote using these gates **in order**:

### Gate 1: Identity Gate
- Pass if `match_confidence_bps >= 5000` (50%)
- Reject quotes that don't match the candidate well enough

### Gate 2: Usability Gate
- Pass if `quote_usability_confidence_bps >= 5000` OR (NULL AND price_signal_role != 'uk_retail_benchmark')
- Retail benchmarks need usability validation; wholesale floors get default-pass on usability

### Gate 3: Realism Gate
- Pass if `quote_realism_confidence_bps >= 4000` OR NULL
- Filter out absurdly cheap quotes (likely scams or wrong product)

### Selection rule:
After applying all 3 gates, if multiple quotes pass, select the quote with the **lowest effective cost** (post-discount). The comparison must use effective cost, not raw price, because the wholesale discount fundamentally changes the apples-to-apples comparison:

```rust
// Compute effective cost for each passing quote, then pick min
let effective = passing_quotes.iter().min_by_key(|q| match q.price_signal_role.as_str() {
    "uk_retail_benchmark" => q.unit_price_gbp_pence * (10_000 - DEFAULT_UK_RETAIL_WHOLESALE_DISCOUNT_BPS) / 10_000,
    _ => q.unit_price_gbp_pence,  // wholesale and CN floor are already trade prices
});
```

**Concrete example (DGU candidate, 5 quotes):**
- Alibaba £5.16 (cn_supplier_floor) → effective cost 516p
- Alibaba £5.79 (cn_supplier_floor) → effective cost 579p
- eBay £18.50 (uk_retail_benchmark) → effective cost 1295p (after 30% discount)
- eBay £19.00 (uk_retail_benchmark) → effective cost 1330p
- eBay £445.00 (uk_retail_benchmark) → effective cost 31150p

**Selected:** Alibaba £5.16 (lowest effective cost).

If NO quotes pass all 3 gates: skip this candidate — no assessment row created. Log a warning.

---

## PHASE 3: GROSS MARGIN COMPUTATION

```rust
let gross_margin_bps: i64 = if revenue == 0 {
    0  // Cannot compute margin without revenue
} else {
    let margin_fraction = (revenue - effective_cost) as f64 / revenue as f64;
    (margin_fraction * 10_000.0) as i64  // can be negative if cost > revenue
};

// Confidence: minimum of revenue confidence and quote confidence
let gross_estimate_confidence_bps = std::cmp::min(
    revenue_confidence_bps,
    quote.match_confidence_bps.unwrap_or(0).min(quote.quote_usability_confidence_bps.unwrap_or(7000))
);

// Pricing input confidence: combined signal
let pricing_input_confidence_bps = (revenue_confidence_bps + gross_estimate_confidence_bps) / 2;
```

### Gate Decision (informational, not a row filter):

| `gross_margin_bps` | `pricing_input_confidence_bps` | Interpretation |
|--------------------|-------------------------------|----------------|
| >= 3000 | >= 6000 | PASS — viable opportunity |
| >= 3000 | < 6000 | REVIEW — margin OK but confidence low |
| < 3000 | * | REJECT — margin insufficient |
| negative | * | REJECT — would lose money |

This gate decision is implicit in the stored values, NOT a separate column. Stage 12 (Net Margin Gate) and Stage 14 (Manifest) read these values to derive the actual decision.

---

## PHASE 4: BUILD `run_gross_margin_gate_stage()`

### Input query:
Find candidates that:
- Have at least 1 `supplier_quotes` row
- Belong to a lot that has `qualify lot` stage head completed
- Are NOT services (`pc.candidate_type != 'service'`) — defensive filter (Stage 9 already excludes these but be explicit)
- Do NOT already have a `gross margin gate` stage head

```sql
SELECT DISTINCT pc.id, pc.process_id, pc.notice_id, pc.lot_id, pc.candidate_value_share_bps
FROM product_candidates pc
JOIN supplier_quotes sq ON sq.product_candidate_id = pc.id
WHERE pc.candidate_type != 'service'
  AND EXISTS (
    SELECT 1 FROM process_stage_heads h
    WHERE h.process_id = pc.process_id AND h.stage = 'qualify lot'
      AND h.scope_kind = 'lot' AND h.scope_id = pc.lot_id
  )
  AND NOT EXISTS (
    SELECT 1 FROM process_stage_heads h
    WHERE h.process_id = pc.process_id AND h.stage = 'gross margin gate'
      AND h.scope_kind = 'candidate' AND h.scope_id = pc.id
  )
```

### Counting candidates per lot/process (for equal-split allocation):

```sql
-- Count total non-service candidates in the lot (for lot-equal-split)
SELECT COUNT(*) FROM product_candidates WHERE lot_id = ?1 AND candidate_type != 'service';

-- Count total non-service candidates in the process (for contract-equal-split)
SELECT COUNT(*) FROM product_candidates WHERE process_id = ?1 AND candidate_type != 'service';
```

Use ALL non-service candidates (not just quoted ones) in the denominator — the contract value is conceptually shared across the full product mix, regardless of which ones we found quotes for.

### Required INSERT fields (all NOT NULL):

| Field | Source / Value |
|-------|----------------|
| `id` | `format!("pricing-{candidate_id}")` |
| `process_id` | from candidate |
| `notice_id` | from candidate |
| `product_candidate_id` | candidate id |
| `selected_quote_id` | id of the selected quote (after lex gating) |
| `selected_quote_row_version` | `quote.row_version` (read from `supplier_quotes.row_version`) — required for downstream staleness detection |
| `quote_cost_boundary` | `'unit'` for unit-priced quotes (use `'pack'` only if the quote represents a multi-unit pack price; default to `'unit'`) |
| `revenue_basis_type` | `'expected_value'` (rules 1-3) or `'rfq_required'` (rule 4) |
| `expected_addressable_revenue_ex_vat_pence` | computed per Phase 1 |
| `revenue_confidence_bps` | per Phase 1 (8500 / 6500 / 4500 / 1000) |
| `gross_margin_bps` | per Phase 3 (can be negative; 0 when revenue=0) |
| `gross_estimate_confidence_bps` | per Phase 3 |
| `identity_gate_passed` | 1 if gate passed, 0 otherwise |
| `usability_gate_passed` | 1 if gate passed, 0 otherwise |
| `pricing_input_confidence_bps` | per Phase 3 |
| `owner_id` | `LOCAL_EXECUTOR_ID` (or `DEFAULT_OWNER_ID` matching the convention used in `run_prequalify_stage`) |
| `last_local_seq` | `next_commit_seq(tx)?` |
| `stage_run_id` | the stage's `allocate_stage_run_id()` value |

### Stage name and scope:
- `persist_stage()` arg: `"gross margin gate"` (lowercase, matches the deterministic pipeline convention)
- `scope_kind`: `"candidate"`
- `scope_id`: the `product_candidate_id`

### Wire into pipeline:

In `run_deterministic_pipeline()`, add after `run_score_quoteability_stage()`:
```rust
let margins_assessed = run_gross_margin_gate_stage(state_db)?;
::tracing::info!(margins_assessed, "gross-margin-gate stage complete");
```

### Idempotency:
Use `ON CONFLICT(id) DO UPDATE` on the INSERT so re-runs refresh the row when quotes change. Update all margin-related columns + `row_version`, but keep the `id` and identifying columns.

### Per-candidate logging (recommended):
For each assessment computed, log a one-line summary so the operator can audit:
```rust
::tracing::info!(
    candidate_id = %pc.id,
    selected_source = %quote.source_name,
    cost_pence = effective_cost,
    revenue_pence = revenue,
    margin_bps = gross_margin_bps,
    "gross margin assessment"
);
```

### Data reality (current DB state):

Before running, know what to expect with the current 4 quoted candidates:

| Candidate | Contract Value | Lot Value | Expected `revenue_basis_type` |
|-----------|---------------:|----------:|-------------------------------|
| Aggregates (CPU 8309) | £1,505,000 | NULL | `expected_value` (rule 3 — equal split across process candidates, ~10 candidates) |
| Double-glazed unit | NULL | NULL | `rfq_required` (rule 4) — revenue=0, margin=0 |
| Library carpet (Ely) | NULL | NULL | `rfq_required` (rule 4) |
| Library carpet/flooring package | NULL | NULL | `rfq_required` (rule 4) |

**This means 3 of 4 assessments will have margin=0** because there's no contract value to compute against. This is correct behavior — the cost data is captured but margin can't be computed without revenue. Stage 10 is doing its job; the upstream issue is that ProContract notices often don't expose contract values.

The `rfq_required` rows still serve a purpose: they record the cost basis (selected supplier quote), so when revenue arrives later (e.g., from manual operator entry or downstream document extraction), Stage 10 can be re-run and produce real margins.

---

## PHASE 5: VALIDATION (run all 10 checks)

### Check 10.1: Assessments created for all quoted candidates
```bash
sqlite3 state/state.db "
SELECT 'Quoted candidates: ' || COUNT(DISTINCT product_candidate_id) FROM supplier_quotes;
SELECT 'Assessments created: ' || COUNT(*) FROM candidate_pricing_assessments;
"
```
**PASS:** Assessments count ≈ quoted candidates count (some may be skipped if no quote passes gates).

### Check 10.2: All assessments have valid quotes selected
```bash
sqlite3 state/state.db "
SELECT 'Orphan assessments (quote missing): ' || COUNT(*) FROM candidate_pricing_assessments cpa
  LEFT JOIN supplier_quotes sq ON sq.id = cpa.selected_quote_id WHERE sq.id IS NULL;
"
```
**PASS:** 0 orphans.

### Check 10.3: Margins computed (split by revenue basis)
```bash
sqlite3 state/state.db "
SELECT
  revenue_basis_type,
  CASE
    WHEN gross_margin_bps >= 5000 THEN 'high (>=50%)'
    WHEN gross_margin_bps >= 3000 THEN 'medium (30-50%)'
    WHEN gross_margin_bps > 0 THEN 'low (0-30%)'
    WHEN gross_margin_bps = 0 THEN 'zero (rfq_required or break-even)'
    ELSE 'negative (loss)'
  END as margin_band,
  COUNT(*)
FROM candidate_pricing_assessments
GROUP BY revenue_basis_type, margin_band ORDER BY revenue_basis_type, margin_band;
"
```
**PASS:** At least 1 assessment exists. With current data, expect ~3 `rfq_required` rows (margin=0) and ~1 `expected_value` row (Aggregates) with a real margin. The Aggregates margin should be substantial — its £1.5M contract divided across ~10 candidates gives ~£150K per candidate vs cheap aggregate quotes, likely a high margin.

### Check 10.4: Wholesale discount applied to retail quotes
```bash
sqlite3 state/state.db "
SELECT cpa.id, sq.unit_price_gbp_pence as raw_price, sq.price_signal_role,
  cpa.expected_addressable_revenue_ex_vat_pence as revenue,
  cpa.gross_margin_bps
FROM candidate_pricing_assessments cpa
JOIN supplier_quotes sq ON sq.id = cpa.selected_quote_id
WHERE sq.price_signal_role = 'uk_retail_benchmark';
"
```
**PASS:** For retail benchmark quotes, the implied cost (`revenue * (10000 - margin_bps) / 10000`) should be ~70% of the raw price. Verify by sampling.

### Check 10.5: DGU candidate selected the cheapest viable quote
```bash
sqlite3 state/state.db "
SELECT cpa.gross_margin_bps, cpa.revenue_basis_type, sq.source_name,
  sq.unit_price_gbp_pence as raw_price_pence, sq.price_signal_role
FROM candidate_pricing_assessments cpa
JOIN supplier_quotes sq ON sq.id = cpa.selected_quote_id
WHERE cpa.product_candidate_id LIKE '%dgu-bs-en-1279%';
"
```
**PASS conditions:**
- DGU has exactly 1 assessment row (it's 1 candidate even though it has 5 quotes)
- `selected_quote.source_name` = `'alibaba-search'` AND `unit_price_gbp_pence` = `516` — this is the lowest effective cost across all 5 quotes (Alibaba £5.16 wins; even the cheapest eBay £18.50 becomes £12.95 after the 30% discount, still higher than £5.16)
- `revenue_basis_type` = `'rfq_required'` and `gross_margin_bps` = `0` — DGU has NULL contract/lot value so margin can't be computed; the assessment captures the cost basis for later re-run when revenue arrives
- This check verifies the **quote selection logic** is working correctly even when the margin computation is blocked on missing revenue.

### Check 10.6: Revenue allocation is sensible
```bash
sqlite3 state/state.db "
SELECT cpa.expected_addressable_revenue_ex_vat_pence, cpa.revenue_basis_type,
  cpa.revenue_confidence_bps, cn.estimated_value_original_minor as contract_value,
  pc.name
FROM candidate_pricing_assessments cpa
JOIN product_candidates pc ON pc.id = cpa.product_candidate_id
JOIN contract_notices cn ON cn.id = cpa.notice_id
LIMIT 10;
"
```
**PASS:** Revenue values are positive (or 0 for `rfq_required`), in pence, and proportional to contract values. No revenue should exceed the contract value.

### Check 10.7: Gate flags populated
```bash
sqlite3 state/state.db "
SELECT identity_gate_passed, usability_gate_passed, COUNT(*)
FROM candidate_pricing_assessments
GROUP BY identity_gate_passed, usability_gate_passed;
"
```
**PASS:** All assessments have both gates set (1 or 0). No NULLs.

### Check 10.8: Stage tracking
```bash
sqlite3 state/state.db "
SELECT stage, scope_kind, COUNT(*) FROM process_stage_heads WHERE stage = 'gross margin gate' GROUP BY stage, scope_kind;
SELECT stage, state, COUNT(*) FROM process_stage_status WHERE stage = 'gross margin gate' GROUP BY stage, state;
"
```
**PASS:** Stage heads exist with `scope_kind = 'candidate'`. State = `succeeded`.

### Check 10.9: FK integrity
```bash
sqlite3 state/state.db "
SELECT 'Orphan assessments (process):  ' || COUNT(*) FROM candidate_pricing_assessments cpa LEFT JOIN procurement_processes pp ON pp.id = cpa.process_id WHERE pp.id IS NULL;
SELECT 'Orphan assessments (notice):   ' || COUNT(*) FROM candidate_pricing_assessments cpa LEFT JOIN contract_notices cn ON cn.id = cpa.notice_id WHERE cn.id IS NULL;
SELECT 'Orphan assessments (candidate):' || COUNT(*) FROM candidate_pricing_assessments cpa LEFT JOIN product_candidates pc ON pc.id = cpa.product_candidate_id WHERE pc.id IS NULL;
SELECT 'Orphan assessments (quote):    ' || COUNT(*) FROM candidate_pricing_assessments cpa LEFT JOIN supplier_quotes sq ON sq.id = cpa.selected_quote_id WHERE sq.id IS NULL;
"
```
**PASS:** All 0.

### Check 10.10: Build status
```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
```
**PASS:** Both clean.

---

## PHASE 6: FIX LOOP

If any check fails:

1. Read the error / unexpected result
2. Diagnose: code bug? data issue? logic error? FK violation?
3. Fix the root cause
4. To re-run Stage 10:
   ```bash
   sqlite3 state/state.db "
   DELETE FROM process_stage_heads WHERE stage = 'gross margin gate';
   DELETE FROM process_stage_status WHERE stage = 'gross margin gate';
   DELETE FROM candidate_pricing_assessments;
   "
   cargo build --release --bin daemon
   cargo run --release --bin daemon -- --pipeline procontract \
     --state-root state --runtime-dir runtime --artifact-root artifacts
   ```
5. Re-run all 10 validation checks
6. Repeat until green

### Common failure patterns:

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| 0 assessments created | Query didn't match any candidates | Check JOIN logic — supplier_quotes + qualify_lot head + no gross_margin_gate head |
| FK violation on insert | `notice_id` mismatch between candidate and quote | Use `pc.notice_id` not `sq.notice_id` (they should match but be explicit) |
| Negative margins everywhere | Forgot to apply wholesale discount | Check that retail benchmark quotes get the 30% discount |
| Revenue all zeros | `estimated_value_original_minor` is NULL on contracts | Allocation rule 4 (`rfq_required`) should kick in — verify it does |
| Margins > 100% | Cost is 0 or sign error | Check unit conversion (pence vs pounds) |
| Stage runs twice and creates duplicates | Missing `ON CONFLICT` | Add `ON CONFLICT(id) DO UPDATE` to the INSERT |

---

## DELIVERABLES

1. **`run_gross_margin_gate_stage()`** function in `crates/daemon/src/main.rs`
2. **Wired into `run_deterministic_pipeline()`** between score_quoteability and relate
3. **All 10 validation checks pass**
4. **`cargo check && cargo clippy`** clean
5. **Mandatory results manifest**

### MANDATORY: Write Results Manifest

After all checks pass, write to `docs/pipeline/stage-10-validation-manifest.md`:

```markdown
# Stage 10 Validation Manifest — Gross Margin Gate

> Generated: {timestamp}
> Status: {GREEN / YELLOW / RED}

## Build Summary
- Function: `run_gross_margin_gate_stage()` at main.rs:LINE
- Wiring: `run_deterministic_pipeline()` invokes after score_quoteability
- Reads: supplier_quotes + product_candidates + notice_lots + contract_notices
- Writes: candidate_pricing_assessments, process_stage_status, process_stage_heads

## Assessment Coverage

| Metric | Value |
|--------|-------|
| Quoted candidates (input) | X |
| Assessments created | X |
| Candidates skipped (no quote passed gates) | X |

## Margin Distribution

| Band | Count | Avg Margin (bps) |
|------|------:|-----------------:|
| high (≥50%) | X | X |
| medium (30-50%) | X | X |
| low (0-30%) | X | X |
| negative (loss) | X | X |

## Per-Candidate Results

| Candidate | Selected Quote | Cost (pence) | Revenue (pence) | Margin (bps) | Decision |
|-----------|---------------|------------:|---------------:|------------:|----------|
| Double-glazed unit | source/price | X | X | X | PASS/REVIEW/REJECT |
| Aggregates | source/price | X | X | X | PASS/REVIEW/REJECT |
| Library carpet × 2 | source/price | X | X | X | PASS/REVIEW/REJECT |

## Wholesale Discount Verification (UK retail benchmarks)

For each retail-benchmark quote selected, verify the effective cost ≈ raw_price × 0.70.

| Quote | Raw Price (pence) | Effective Cost (pence) | Discount Applied |
|-------|------------------:|----------------------:|------------------|
| ... | X | X | YES (30%) |

## Validation Checks

| # | Check | Result | Detail |
|---|-------|--------|--------|
| 10.1 | Assessments for quoted candidates | PASS/FAIL | X |
| 10.2 | Quote FK valid | PASS/FAIL | X |
| 10.3 | Margin distribution | PASS/FAIL | X |
| 10.4 | Wholesale discount | PASS/FAIL | X |
| 10.5 | DGU dual-source pick | PASS/FAIL | X |
| 10.6 | Revenue allocation | PASS/FAIL | X |
| 10.7 | Gate flags | PASS/FAIL | X |
| 10.8 | Stage tracking | PASS/FAIL | X |
| 10.9 | FK integrity | PASS/FAIL | X |
| 10.10 | Build status | PASS/FAIL | X |

## Code Changes
| File | Change |
|---|---|
| `crates/daemon/src/main.rs` | Confirmed `run_gross_margin_gate_stage()` exists at the current line number and is wired into `run_deterministic_pipeline()` |

## Issues Found & Fixed
{For each: what failed, root cause, fix, verification}

## Issues Remaining
{Any gaps, or "None"}

## Data Snapshot
{Full output of all validation queries}
```

---

## CONSTRAINTS

1. **Work in a loop until green.** Do NOT report "done" with failing checks.
2. **Stage 10 is deterministic code only.** No LLM, no agent skill.
3. **Follow existing patterns.** Use `run_prequalify_stage` as template.
4. **Apply the wholesale discount.** Only to `uk_retail_benchmark` quotes. Use the existing `DEFAULT_UK_RETAIL_WHOLESALE_DISCOUNT_BPS` constant — don't redefine it.
5. **Idempotent INSERT.** Use `ON CONFLICT(id) DO UPDATE`.
6. **Skip candidates with no passing quote.** Don't create an assessment if no quote clears the 3 gates.
7. **Rust conventions:** `cargo check --all-targets && cargo clippy --all-targets -- -D warnings` must pass.
8. **Do NOT build Stages 11-12.** Stage 10 ends at gross margin assessments. Landed cost (Stage 11) and Net Margin Gate (Stage 12) are separate.

---

## REMINDERS

- ULTRATHINK and be meticulous.
- **The loop is non-negotiable.** Run → validate → fix → repeat until green.
- Only 4 candidates have quotes today. Stage 10 will produce ≤4 assessments. That's fine — the pipeline scales when more marketplace runs add more quotes.
- **DGU showcase reality check:** the DGU candidate has 5 quotes (the largest set, including a UK+CN pair £18.50/£5.16) AND it is the technical showcase for quote selection — Stage 10 will correctly select the £5.16 Alibaba quote as the lowest effective cost. **BUT** the DGU's contract has NULL `estimated_value_original_minor` AND NULL `notice_lots.estimated_value_pence`, so revenue=0, `revenue_basis_type='rfq_required'`, and `gross_margin_bps=0`. This is correct — the cost selection is the win; the margin computation is blocked on missing revenue. The cost-selection logic still gets exercised and verified by Check 10.5.
- **The Aggregates candidate is the only one that will produce a real margin.** Its contract value is £1,505,000 (150500000 pence) shared across ~10 candidates → ~£150K addressable revenue per candidate. Against an aggregate cost of a few pence per kg, this should yield a very high `gross_margin_bps` (likely 9000+).
- If a candidate has a quote but no contract value, `revenue_basis_type = 'rfq_required'` and the margin is unreliable — record it but mark low confidence.
- The 30% wholesale discount is operator-tunable. Don't hardcode 3000 in your code — read `DEFAULT_UK_RETAIL_WHOLESALE_DISCOUNT_BPS`.
