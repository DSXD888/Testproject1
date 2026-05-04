# Full Pipeline E2E Commercial Basis Follow-up Manifest

> Generated: 2026-05-04T20:24:06Z
> Status: GREEN for code, schema, and regression gates. Live marketplace re-drive was not run in this session; the next isolated E2E run should execute `--ingest marketplace-coverage` against `state-e2e-20260504T113157Z` or a fresh copy before claiming live Alibaba coverage.
> Spotlight process: `proc-pc-956d1aaa-8841-f111-813c-005056b64545`

## Mission Recap

The Woodhouses E2E run exposed a cross-stage commercial-basis defect: Stage 8 noticed boiler quantity evidence but did not persist candidate quantities, Stage 9 stopped after one UK-side eBay quote without durable Alibaba/import evidence, Stage 10 treated that one-sided quote as normal gross-margin input, and Stage 11 inferred an unsafe `42.3908435777872` boiler quantity from whole lot value divided by the eBay unit quote.

This follow-up implements the corrected state machine:

- Stage 8 can persist explicit product quantities from structured `quantity` fields or supported `quantity_hint` evidence.
- Stage 9 now has durable quote-coverage rows for required commercial roles and a coverage orchestration entry point.
- Stage 10 distinguishes two-sided supplier comparison from fallback screening and from missing required role attempts.
- Stage 11 refuses whole-lot value quantity derivation when Stage 10 used allocated candidate revenue.
- Stage 12/13 surfaces missing import attempts, fallback screening, incomplete quote coverage, and quantity-basis issues.

## Woodhouses Failure Answers

| Question | Answer |
|---|---|
| Why is `/tmp/e2e_woodhouses_alibaba_job.json` not durable Stage 9 evidence? | It is only prepared input. Durable evidence requires database rows such as `workflow_jobs`, `agent_session_logs`, `search_attempts`, `supplier_quotes`, and now `candidate_quote_coverage`. |
| Which tables proved Alibaba/GEM/MX did not complete? | `search_attempts` had no `alibaba-search`, `gem-imports`, or `mx-wholesale` rows for the Woodhouses B01/B02 candidates, and `supplier_quotes` only had `ebay-uk`. |
| Which DB fields showed missed quantity? | `product_candidates.quantity` and `product_candidates.quantity_unit` were `NULL` for B01/B02 even while `quantity_confidence_bps` remained high. |
| Why did Stage 10 allocate GBP 65,000 to B02? | The gross-margin revenue allocator split the GBP 130,000 lot value across two non-service candidates. |
| Why did Stage 11 infer `42.39` boilers? | Candidate quantity was missing, so the landed-cost quantity ladder divided the whole lot value by the selected eBay unit quote even though Stage 10 had used only half the lot value as B02 revenue. |

## Implemented Changes

| Area | Change |
|---|---|
| Schema | Registered migration `0011_commercial_basis_coverage` and bumped `DEFAULT_SCHEMA_EPOCH` to 8. Added `candidate_quote_coverage` plus commercial-basis columns on `candidate_pricing_assessments`. |
| Stage 8 | Aligned tender-spec skill schema, prompt, and daemon persistence. Explicit numeric `quantity`, `quantity_unit`, `quantity_confidence_bps`, and parseable `quantity_hint` values now flow into `product_candidates`. Woodhouses-style total `2no boilers` plus B01/B02 schedule references allocate as 1 unit each; ambiguous totals do not invent quantities. |
| Stage 9 | Added required role coverage for `uk_side_quote` and `china_import_quote`. `--ingest marketplace-coverage` runs Amazon UK, eBay UK, GEM, MX, then Alibaba. Marketplace result contracts update both `search_attempts` and `candidate_quote_coverage`. A no-result-contract agent failure now persists `search_attempts.search_failure_mode='AGENT_RUN_FAILED'` and a `candidate_quote_coverage` attempted-failed row. |
| Stage 10 | Reworked gross-margin selection around role coverage. Normal basis requires UK-side and China/import-side attempts plus usable quotes. Two-sided input records `two_sided_supplier_comparison`, quote IDs, spread, and normal coverage. Single UK quote before Alibaba/import attempt records `missing_required_quote_role_attempt`; single-side discount is only fallback after import has been durably attempted and failed/no-quoted. |
| Stage 11 | Added commercial-basis fields to landed-cost inputs and confidence JSON. Candidate quantity wins. Whole-lot value derived quantity is rejected when candidate revenue and lot value do not align. The Woodhouses B02 `42.39` path is now blocked into `basis_undetermined` unless reliable quantity evidence exists. |
| Stage 12/13 | Net gate and manifest reasons now expose missing required quote role attempts, missing China/import quote after attempt, fallback screening basis, incomplete coverage, and quantity ambiguity. |

## Regression Coverage

| Test | Coverage |
|---|---|
| `woodhouses_boiler_quantity_hint_persists_candidate_quantities` | B01/B02 plus total `2no boilers` becomes 1 unit each. |
| `ambiguous_total_quantity_does_not_allocate_candidate_quantity` | Total quantity that cannot map to candidates remains unallocated. |
| `marketplace_agent_failure_persists_import_coverage_attempt` | Alibaba no-result-contract failure writes durable attempt and quote-coverage state. |
| `gross_margin_blocks_normal_basis_when_import_source_not_attempted` | eBay-only input cannot become normal commercial readiness. |
| `gross_margin_single_uk_quote_is_fallback_after_import_attempt_failure` | Single UK quote uses labelled screening fallback only after durable import failure. |
| `gross_margin_prefers_two_sided_supplier_comparison` | UK-side plus Alibaba quote selects the China/import supply quote and records normal two-sided basis. |
| `stage11_uses_extracted_candidate_quantity_before_lot_value_fallback` | Extracted candidate quantity beats value-derived inference. |
| `stage11_rejects_lot_value_quantity_when_stage10_used_allocated_revenue` | Whole-lot value cannot infer candidate quantity when Stage 10 used allocated revenue. |

## Quality Gates

| Gate | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo check --all-targets` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test -p daemon --bin daemon` | PASS, 202 tests |
| `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done` | PASS, every daemon integration target |

## Live Re-drive Status

No live marketplace re-drive was run here. The implementation deliberately avoids claiming live Alibaba coverage until a real isolated E2E run writes durable rows. The next re-drive should:

```bash
export E2E_STATE_ROOT=state-e2e-20260504T113157Z
export E2E_RUNTIME_DIR=runtime-e2e-20260504T113157Z
export E2E_ARTIFACT_ROOT=artifacts-e2e-20260504T113157Z
target/debug/daemon --ingest marketplace-coverage
target/debug/daemon --ingest gross-margin
target/debug/daemon --ingest landed-cost
target/debug/daemon --ingest net-gate
target/debug/daemon --ingest manifest
```

Expected Woodhouses post-re-drive shape:

- B01/B02 quantities are 1 unit when the Stage 8 semantic result carries the explicit total quantity evidence.
- `alibaba-search` has `search_attempts` rows or `candidate_quote_coverage.coverage_state='attempted_failed'` with an explicit failure mode.
- eBay-only evidence records `commercial_basis_kind='missing_required_quote_role_attempt'` before Alibaba/import is attempted, or labelled fallback after import failure.
- No landed-cost confidence breakdown contains `order_quantity_basis.qty=42.3908435777872`.
- Manifest text shows UK quote evidence, China/import quote state, commercial basis kind, quantity basis, and fallback/screening status.
