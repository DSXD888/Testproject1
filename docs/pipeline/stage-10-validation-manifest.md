# Stage 10 Validation Manifest - Gross Margin Gate

> Generated: 2026-04-25T16:41:41+03:00
> Status: GREEN

## Build Summary
- Function: `run_gross_margin_gate_stage()` at main.rs:11178
- Wiring: `run_deterministic_pipeline()` invokes after score_quoteability at main.rs:9864
- Reads: supplier_quotes + product_candidates + notice_lots + contract_notices
- Writes: candidate_pricing_assessments, process_stage_status, process_stage_heads

## Assessment Coverage

| Metric | Value |
|--------|------:|
| Quoted candidates (input) | 4 |
| Assessments created | 4 |
| Candidates skipped (no quote available) | 0 |

## Margin Distribution

| Band | Count | Avg Margin (bps) |
|------|------:|-----------------:|
| high (>=50%) | 1 | 9999 |
| medium (30-50%) | 0 | 0 |
| low (0-30%) | 0 | 0 |
| zero (rfq_required or break-even) | 3 | 0 |
| negative (loss) | 0 | 0 |

## Per-Candidate Results

| Candidate | Selected Quote | Cost (pence) | Revenue (pence) | Margin (bps) | Decision |
|-----------|----------------|-------------:|----------------:|-------------:|----------|
| Aggregates (general) | amazon-uk / 88p retail, 61p effective | 61 | 16722222 | 9999 | REVIEW |
| Double-glazed unit | alibaba-search / 516p CN floor | 516 | 0 | 0 | REJECT |
| Ely Library carpet / flooring package | amazon-uk / 6495p retail, 4546p effective | 4546 | 0 | 0 | REJECT |
| Library carpet / floor covering | amazon-uk / 8093p retail, 5665p effective | 5665 | 0 | 0 | REJECT |

Decision basis: PASS requires gross_margin_bps >= 3000 and pricing_input_confidence_bps >= 6000. Aggregates has high raw margin but only 5250 pricing input confidence because the selected retail quote failed identity/usability gates, so it is REVIEW.

## Wholesale Discount Verification (UK retail benchmarks)

For each selected retail-benchmark quote, effective cost is raw_price * 0.70 using `DEFAULT_UK_RETAIL_WHOLESALE_DISCOUNT_BPS`.

| Quote | Raw Price (pence) | Effective Cost (pence) | Discount Applied |
|-------|------------------:|-----------------------:|------------------|
| sq-a5e2e2740cde4d01a9ebae18723f832b-0 | 88 | 61 | YES (30%) |
| sq-b63c50f7df934cd6a15a7f7118797df0-0 | 6495 | 4546 | YES (30%) |
| sq-3adffef4dbf049459de21ebeafad67d4-0 | 8093 | 5665 | YES (30%) |

Note: for the aggregates row, inferring cost back from stored `gross_margin_bps` is lossy because the revenue is very large and the stored margin has only 1 bps precision. The direct selected effective cost is 61p, correctly equal to 70% of 88p rounded by integer division.

## Validation Checks

| # | Check | Result | Detail |
|---|-------|--------|--------|
| 10.1 | Assessments for quoted candidates | PASS | 4 quoted candidates, 4 assessments |
| 10.2 | Quote FK valid | PASS | 0 missing selected quotes |
| 10.3 | Margin distribution | PASS | 1 expected_value high-margin row, 3 rfq_required zero-margin rows |
| 10.4 | Wholesale discount | PASS | Selected retail costs are raw * 0.70 |
| 10.5 | DGU dual-source pick | PASS | alibaba-search 516p selected; rfq_required; margin 0 |
| 10.6 | Revenue allocation | PASS | Aggregates allocated 16722222p; rfq rows have 0p |
| 10.7 | Gate flags | PASS | 1 row has gates 1/1, 3 low-confidence rows have gates 0/0 |
| 10.8 | Stage tracking | PASS | 4 candidate stage heads; 4 succeeded statuses |
| 10.9 | FK integrity | PASS | 0 process/notice/candidate/quote orphans |
| 10.10 | Build status | PASS | cargo fmt --check, cargo check --all-targets, cargo clippy --all-targets -- -D warnings |

## Code Changes

| File | Change |
|------|--------|
| `crates/daemon/src/main.rs` | Added `run_gross_margin_gate_stage()` and helper structs/functions; wired into `run_deterministic_pipeline()` |
| `docs/pipeline/stage-10-validation-manifest.md` | Added this validation manifest |

## Issues Found & Fixed

| Issue | Root Cause | Fix | Verification |
|-------|------------|-----|--------------|
| Strict quote gates produced only 1 assessment | Current Stage 9 retail quotes have NULL usability confidence; aggregates/carpet retail quotes also have identity confidence below 5000 | Stage 10 now prefers fully passing quotes, then falls back to the best available quote while storing identity/usability gate flags as 0 | Pipeline rerun created 4 assessments and check 10.7 shows the low-confidence rows explicitly |

## Issues Remaining

Retail-only fallback assessments are low-confidence until Stage 9 populates stronger identity/usability confidence for retail benchmarks. The rows are still useful because they preserve the selected cost basis and make the failed quote gates visible.

## Data Snapshot

```text
Check 10.1
result
--------------------
Quoted candidates: 4
result
----------------------
Assessments created: 4

Check 10.2
result
-------------------------------------
Orphan assessments (quote missing): 0

Check 10.3
revenue_basis_type  margin_band                        count
------------------  ---------------------------------  -----
expected_value      high (>=50%)                       1
rfq_required        zero (rfq_required or break-even)  3

Check 10.4
id                                                                                                      raw_price  price_signal_role    revenue   gross_margin_bps  expected_effective_cost  implied_effective_cost
------------------------------------------------------------------------------------------------------  ---------  -------------------  --------  ----------------  -----------------------  ----------------------
pricing-pc-d23508ec-853d-f111-813c-005056b64545-lot-implicit-candidate-cpu8309-lot1-aggregates-general  88         uk_retail_benchmark  16722222  9999              61                       1672
pricing-pc-19574d11-0929-f111-813c-005056b64545-lot-1-candidate-ely-library-design-fit-out-lot1-carpet  6495       uk_retail_benchmark  0         0                 4546
pricing-pc-19574d11-0929-f111-813c-005056b64545-lot-1-candidate-ely-library-carpet                      8093       uk_retail_benchmark  0         0                 5665

Check 10.5
gross_margin_bps  revenue_basis_type  source_name     raw_price_pence  price_signal_role
----------------  ------------------  --------------  ---------------  -----------------
0                 rfq_required        alibaba-search  516              cn_supplier_floor

Check 10.6
expected_addressable_revenue_ex_vat_pence  revenue_basis_type  revenue_confidence_bps  contract_value  name
-----------------------------------------  ------------------  ----------------------  --------------  -------------------------------------------------------------------------------------------------------
0                                          rfq_required        1000                                    Double-glazed unit (sealed insulating glass unit, BS EN 1279-compliant, argon-filled, warm-edge spacer)
16722222                                   expected_value      6500                    150500000       Aggregates (general, framework-wide, detailed grades in ITT)
0                                          rfq_required        1000                                    Ely Library carpet / flooring package
0                                          rfq_required        1000                                    Library carpet / floor covering (Ely Library refresh)

Check 10.7
identity_gate_passed  usability_gate_passed  count
--------------------  ---------------------  -----
0                     0                      3
1                     1                      1

Check 10.8
stage              scope_kind  count
-----------------  ----------  -----
gross margin gate  candidate   4
stage              state      count
-----------------  ---------  -----
gross margin gate  succeeded  4

Check 10.9
result
--------------------------------
Orphan assessments (process):  0
result
--------------------------------
Orphan assessments (notice):   0
result
--------------------------------
Orphan assessments (candidate):0
result
--------------------------------
Orphan assessments (quote):    0

Per-candidate detail
candidate                                                                                                source_name     raw_cost_pence  effective_cost_pence  revenue_pence  gross_margin_bps  pricing_input_confidence_bps  identity_gate_passed  usability_gate_passed  revenue_basis_type
-------------------------------------------------------------------------------------------------------  --------------  --------------  --------------------  -------------  ----------------  ----------------------------  --------------------  ---------------------  ------------------
Aggregates (general, framework-wide, detailed grades in ITT)                                             amazon-uk       88              61                    16722222       9999              5250                          0                     0                      expected_value
Double-glazed unit (sealed insulating glass unit, BS EN 1279-compliant, argon-filled, warm-edge spacer)  alibaba-search  516             516                   0              0                 1000                          1                     1                      rfq_required
Ely Library carpet / flooring package                                                                    amazon-uk       6495            4546                  0              0                 1000                          0                     0                      rfq_required
Library carpet / floor covering (Ely Library refresh)                                                    amazon-uk       8093            5665                  0              0                 1000                          0                     0                      rfq_required
```
