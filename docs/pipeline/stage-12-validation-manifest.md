# Stage 12 Net Margin Gate Validation Manifest

> Generated: 2026-05-01T10:46:19Z
> Status: GREEN

## Summary
Stage 12 net margin gate was built end-to-end against the live SQLite DB: migration 0007 adds `candidate_net_margin_assessments`, `--ingest net-gate` writes 17 per-candidate verdicts, and `pipeline_assessments` now has 7 `net_qualified` process rollups for Stage 13. Live verdict distribution is 0 PASS / 17 REVIEW / 0 REJECT, with 4 priced REVIEW rows and 13 searched-unpriced `no_pricing_signal` REVIEW rows. Pre-flight drift observed: service-excluded candidates are 172 today, not 3; root migrations are 0001/0003/0004/0005/0006 before 0007; Stage 11 has both fixture and live ID paths, but the verified live inputs still produce the required 4 priced + 13 searched-unpriced Stage 12 set.

## Build Summary
- Migration: `migrations/0007_net_margin_gate_assessments.sql`
- Migration registered: `crates/daemon/src/db/sqlite.rs` MIGRATIONS array (epoch 4)
- Code: `run_net_margin_gate_live_loop` at `main.rs:6589`
- Dispatch: `--ingest net-gate` at `main.rs:1172`
- New unit tests: T.1-T.18 (all in `mod tests`)
- New schema test: `ngg_t_schema_activation_007_*` in `crates/daemon/tests/schema_activation.rs`

## Live Run Result
```text
net-margin-gate live loop completed: attempted=17 candidates_assessed=17 pass=0 review=17 reject=0 no_pricing_signal=13 processes_rolled_up=7 skipped_unchanged=0
net-margin-gate live loop completed: attempted=17 candidates_assessed=0 pass=0 review=0 reject=0 no_pricing_signal=0 processes_rolled_up=0 skipped_unchanged=17
```

## Per-Candidate Verdict Distribution
| verdict | count | sample reasons |
|---|---:|---|
| review | 17 | `["awaiting_rfq_quote","confidence_commercial_estimate_below_floor","confidence_pricing_input_below_floor","confidence_revenue_below_floor"]` |

## Per-Process Rollup
| process_id | pipeline_state | review_required | priced_coverage_bps | landed_cost_coverage_bps | net_qualified_coverage_bps | unresolved_mandatory_lots | review_reason |
|---|---|---:|---:|---:|---:|---:|---|
| `proc-pc-19574d11-0929-f111-813c-005056b64545` | `net_qualified` | 1 | 10000 | 10000 | 0 | 0 | `awaiting_rfq_quote` |
| `proc-pc-218726bd-9dda-f011-813c-005056b64545` | `net_qualified` | 1 | 0 | 0 | 0 | 9 | `no_pricing_signal` |
| `proc-pc-3016d0da-ef3c-f111-813c-005056b64545` | `net_qualified` | 1 | 0 | 0 | 0 | 1 | `no_pricing_signal` |
| `proc-pc-3ba4e6b8-9cef-f011-813c-005056b64545` | `net_qualified` | 1 | 0 | 0 | 0 | 1 | `no_pricing_signal` |
| `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` | `net_qualified` | 1 | 5000 | 5000 | 0 | 1 | `quantity_basis_unaligned` |
| `proc-pc-7e2a6fb2-67a3-f011-813a-005056b64545` | `net_qualified` | 1 | 0 | 0 | 0 | 5 | `no_pricing_signal` |
| `proc-pc-d23508ec-853d-f111-813c-005056b64545` | `net_qualified` | 1 | 5000 | 5000 | 0 | 1 | `quantity_basis_undetermined` |

## R.1-R.18 Acceptance Criteria
| # | Criterion | Result | Detail |
|---|---|---|---|
| R.1 | Candidate row count | PASS | 17 rows = 4 priced + 13 searched-unpriced |
| R.2 | Verdict distribution | PASS | 0 pass, 17 review, 0 reject |
| R.3 | Reason enum closed | PASS | Only awaiting RFQ, confidence floor, quantity basis, and no-pricing tokens emitted |
| R.4 | Aggregates quantity basis | PASS | `review`, `["quantity_basis_undetermined"]`, re-evaluation eligible |
| R.5 | RFQ-required rows | PASS | 3 priced RFQ rows include `awaiting_rfq_quote`, all REVIEW |
| R.6 | No-pricing rows | PASS | 13 exact `["no_pricing_signal"]`; margin and confidence fields NULL |
| R.7 | Confidence floor bidirectional check | PASS | All 8 missing/spurious reason counters are 0 |
| R.8 | Pipeline assessment count/FK | PASS | 7 rows = 7 distinct Stage 12 processes; no process FK orphans |
| R.9 | Pipeline assessment state/review | PASS | All `net_qualified`, `review_required=1`, `review_stage=net_margin_gate` |
| R.10 | Coverage metrics bounds | PASS | 0 rows out of `[0,10000]` bounds |
| R.11 | `process_stage_status` rows | PASS | 17 candidate + 7 process rows, all `succeeded` |
| R.12 | `process_stage_heads` rows | PASS | 17 candidate + 7 process heads |
| R.13 | Candidate FK integrity | PASS | 0 process, notice, candidate, pricing, or landed-cost orphans |
| R.14 | Re-runnability | PASS | Executed the handoff R.14 Stage 12 reset at 2026-05-01T10:40:58Z and regenerated the same 17 verdict/reason rows |
| R.15 | Idempotency | PASS | Second invocation skipped all 17 unchanged rows; row count stayed 17 |
| R.16 | Stage 11 untouched | PASS | DGU row remains `review|5160|17692|0|4570|27422` |
| R.17 | Quality gates | PASS | fmt, check, clippy, and 99 binary tests passed |
| R.18 | Per-file integration loop | PASS | 39 `crates/daemon/tests/*.rs` targets passed |

## Quality Gates
- cargo fmt --check: PASS
- cargo check --all-targets: PASS
- cargo clippy --all-targets -- -D warnings: PASS
- cargo test -p daemon --bin daemon: PASS - 99 tests, includes T.1-T.18
- per-file integration loop: PASS - 39 files

## Issues Found & Fixed
- Migration activation failed because 0007 initially used an inner `BEGIN`/`COMMIT` while `apply_sql_file` already wraps migrations in a savepoint -> removed the inner transaction wrapper -> binary unit tests and schema activation passed.
- Clippy flagged bool-to-int conversions, unnecessary `async`, and a needless mutable test fixture parameter -> switched to `i64::from(bool)`, made the net-gate wrapper synchronous, and narrowed the fixture signature -> clippy passed.

## Code Changes
| File | Change |
|---|---|
| `migrations/0007_net_margin_gate_assessments.sql` | Added idempotent `candidate_net_margin_assessments` table and indexes |
| `crates/daemon/src/db/sqlite.rs` | Registered 0007 migration and bumped default schema epoch to 4 |
| `crates/daemon/src/main.rs` | Added `--ingest net-gate`, verdict policy, basis hashing, candidate upsert, rollup upsert, stage tracking, and T.1-T.18 tests |
| `crates/daemon/tests/schema_activation.rs` | Added 0007 upgrade-path test and updated default epoch assertion to 4 |
| `docs/pipeline/stage-12-validation-manifest.md` | Captured live validation results and quality gates |

## Out-of-Scope (Documented, Not Fixed Here)
- Operator override mechanism (Stage 14)
- Wiring `--ingest net-gate` into `run_deterministic_pipeline`
- Lot-level `mandatory` flag (no schema field today)
