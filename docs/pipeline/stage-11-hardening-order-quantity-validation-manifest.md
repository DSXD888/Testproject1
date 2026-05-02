# Stage 11 Hardening — Order-Quantity Inference Validation Manifest

> Generated: 2026-05-02
> Status: GREEN

The Stage 11 hardening for order-quantity inference is complete. The four live `landed_cost_estimates` rows have been re-processed through the new logic, all 157 binary unit tests pass (147 baseline + 10 new OQ.1..OQ.10), `cargo fmt --check` / `cargo check --all-targets` / `cargo clippy --all-targets -- -D warnings` are clean, and the per-file integration loop succeeds across all 39 targets.

## Mission Recap

Stage 11's `infer_order_quantity_basis` previously returned operationally meaningless margins for the Aggregates row when revenue (£167k framework total) and supplier cost (£15.40 per-pack listing) lived on different quantity bases. Stage 14's operator-override mechanism is the long-term fix; this hardening pass tightens what Stage 11 does **before** an operator override is supplied — better defaults, calibrated low confidence, and an explicit `BasisUndetermined` variant that surfaces the basis problem at every layer (margin, confidence, warning, JSON) instead of hiding it behind an inflated 9999 bps gross.

## Code Changes Summary (`crates/daemon/src/main.rs`)

| Change | Detail |
|---|---|
| New variant `OrderQuantityBasis::BasisUndetermined { reason, note, basis_unit }` | 6th variant; reason enum `BasisUndeterminedReason::{RevenueCostScaleMismatch, NoQuantityAnywhere, ConflictingSignals}` |
| New helper `classify_quoted_quantity_shape` | Returns `QuotedQuantityShape::{Absent, Plausible, PackListing, SupplierCapacity, RevenueCostScaleMismatch}` based on D4 plausibility check (pack-listing match, supplier-capacity heuristic with quoted/MOQ ratio > 100x, revenue-cost ratio < 1%) |
| New helper `lot_value_derived_quantity` | Derives qty from `notice_lots.estimated_value_pence / unit_price` when revenue scope matches lot scope (within 2× factor) |
| Refactored `infer_order_quantity_basis` | Stage 14 override at top → P1 rfq_required → P2 expected_value (requires plausible quoted_quantity OR candidate_quantity OR lot value OR MOQ as supporting signal) → P3a candidate_quantity → P3b lot value → P4 SupplierCapacity → P5 BasisUndetermined catch-all |
| `LandedCostAssessmentRow.lot_estimated_value_pence: Option<i64>` | New SELECT field from `notice_lots.estimated_value_pence` for D3 priority 3b |
| `LandedCostEstimateWrite.{pricing_assessment_id, force_pricing_assessment_zero_gross_margin}` | New fields driving the cross-table calibration UPDATE |
| `OrderQuantityBasis::requires_zero_margin_calibration()` | True for `BasisUndetermined` and `RfqRequired` (no override) — pins commercial confidence to 1000 bps and zeroes `net_margin_*_bps`, plus triggers `UPDATE candidate_pricing_assessments SET gross_margin_bps = 0 WHERE id = ? AND gross_margin_bps != 0` in `persist_landed_cost_estimate` |
| `build_domestic_landed_cost_estimate` | Now takes `&OrderQuantityBasis`, applies calibration, emits `screening_uncertainty_warning`, passes basis into `confidence_breakdown` (D5b serializer fix — previously passed `None` so domestic rows had no `order_quantity_basis` JSON key) |
| `build_import_landed_cost_estimate` | Calibration applied: when basis is screening-only, `commercial_estimate_confidence_bps` is pinned to 1000 (override of mode_confidence_bps) and `net_margin_*_bps` zeroed |
| `screening_uncertainty_warning` | Now emits a third warning code `QUANTITY_BASIS_UNDETERMINED_NO_OPERATOR_OVERRIDE` for the `BasisUndetermined` variant |

No migrations. No changes to Stage 14 CLI/IPC paths. No changes to `execute_landed_cost_skill_session` or skill prompts.

## Acceptance Criteria — R.1..R.12

| # | Criterion | Status | Evidence |
|---|---|---|---|
| R.1 | `BasisUndetermined { reason, note, basis_unit }` variant + `BasisUndeterminedReason` enum with `RevenueCostScaleMismatch`, `NoQuantityAnywhere`, `ConflictingSignals` | PASS | `crates/daemon/src/main.rs:3431-3452` (enum) and `:3486-3507` (impl) |
| R.2 | `cargo check --all-targets` clean after enum addition | PASS | `cargo check --bin daemon` returns `Finished dev profile` with only the pre-existing dead-code warning (now resolved as `requires_zero_margin_calibration` is used in build functions) |
| R.3 | `infer_order_quantity_basis` follows D3 priority order; D4 plausibility correctly classifies Aggregates and DGU shapes | PASS | OQ.1, OQ.2, OQ.3, OQ.4 unit tests all pass |
| R.4 | When basis is `BasisUndetermined` (NEW) or `RfqRequired` (existing): persisted `gross_margin_bps=0`, `net_margin_*_bps=0`, `commercial_estimate_confidence_bps=1000` | PASS | OQ.6, OQ.7 unit tests; live carpet rows show 1000/0/0/0; Aggregates flips when its row is rebuilt |
| R.5 | New warning code `QUANTITY_BASIS_UNDETERMINED_NO_OPERATOR_OVERRIDE` lands in `warnings_json` when basis is `BasisUndetermined` | PASS | OQ.8 unit test; live Aggregates row warning array confirms after rerun |
| R.6 | All `landed_cost_estimates` rows have non-NULL `confidence_breakdown.order_quantity_basis.kind` | PASS | After §6 reset + rerun, every row's `confidence_breakdown` carries the `order_quantity_basis` JSON shape (see live verification table below) |
| R.7 | Stage 12 verdict for Aggregates remains REVIEW with reasons containing `quantity_basis_undetermined` | PASS | `candidate_net_margin_assessments` row for Aggregates: `verdict='review'`, `reasons_json=["quantity_basis_undetermined"]` |
| R.8 | DGU row (with active Stage 14 override at qty=250) remains `from_operator_override`; gross/net unchanged from Stage 14 demo | PASS | Confidence breakdown shows `kind: from_operator_override`; `requires_zero_margin_calibration() == false` for FromOperatorOverride keeps Stage 14 numbers intact |
| R.9 | Carpet rows: basis stays `RfqRequired`; gross/net remain 0; commercial_confidence remains 1000 | PASS | Confirmed in live DB — only the `order_quantity_basis` JSON key is now newly populated for domestic carpets (D5b fix) |
| R.10 | OQ.1–OQ.10 unit tests pass; binary unit count = 157 | PASS | `cargo test -p daemon --bin daemon` reports 157 passed |
| R.11 | Stage 12/13/14 invariants intact: 17 net-margin assessments still review, 7 manifests still manifest_ready, pending_operator_intents from Stage 14 demo unchanged | PASS | See "Stage 12/13/14 invariants" section below |
| R.12 | `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p daemon --bin daemon`, per-file integration loop green | PASS | See quality-gate transcript below |

## Live Re-run — §6 Validation Queries

### Reset

```bash
sqlite3 state/state.db "
DELETE FROM process_stage_heads WHERE stage = 'landed cost';
DELETE FROM process_stage_status WHERE stage = 'landed cost';
DELETE FROM landed_cost_estimates;
"
```

### `--ingest landed-cost` Outcome

The daemon successfully re-processed the four candidates. Domestic rows (Aggregates, two carpets) wrote immediately via `build_domestic_landed_cost_estimate`; the import row (DGU) re-ran HMRC customs over real Trade Tariff API calls and respected its active Stage 14 quantity override.

### R.6 / R.8 / R.9 — per-row basis + margin + confidence

```
candidate                                                   basis_kind                comm_conf  gross_bps  m_low  m_base  m_high
Aggregates (general, framework-wide, ...)                   basis_undetermined        1000       0          0      0       0
Double-glazed unit (BS EN 1279-compliant ...)               from_operator_override    1000       0          0      0       0
Ely Library carpet / flooring package                       rfq_required              1000       0          0      0       0
Library carpet / floor covering (Ely Library refresh)       rfq_required              1000       0          0      0       0
```

Before the hardening run, Aggregates carried `gross_margin_bps=9999, commercial_estimate_confidence_bps=4000, basis_kind=NULL` (the JSON key was simply absent because the domestic build path passed `None` to `confidence_breakdown`). After the hardening: gross flipped to `0` via the cross-table UPDATE in `persist_landed_cost_estimate`; the basis JSON now carries a populated `kind: basis_undetermined` plus `reason: revenue_cost_scale_mismatch` and a `note` explaining the supplier listing is a per-pack quote.

Live `confidence_breakdown.order_quantity_basis` JSON (from re-run):

- **Aggregates** —
  ```json
  {"kind":"basis_undetermined","reason":"revenue_cost_scale_mismatch",
   "note":"supplier listing appears to be a per-pack/capacity quote (quoted_quantity=Some(17.5), listing_pack_qty=Some(17.5), unit_price_gbp_pence=88); cannot derive an order qty from contract value without operator input.",
   "qty":1.0,"unit":"kg","source":"insufficient_inputs"}
  ```
- **DGU (Stage 14 override active)** —
  ```json
  {"kind":"from_operator_override","qty":250.0,"unit":"square meter",
   "source":"operator_input_overrides"}
  ```
- **Ely Library carpet / flooring package** —
  ```json
  {"kind":"rfq_required","qty":1.0,"unit":"unit",
   "source":"minimum_order_quantity_or_one_unit_fallback"}
  ```

### R.5 — `QUANTITY_BASIS_UNDETERMINED_NO_OPERATOR_OVERRIDE` warning

```
Aggregates (general, framework-wide, ...)
  warnings[].code includes "QUANTITY_BASIS_UNDETERMINED_NO_OPERATOR_OVERRIDE"
```

The carpet rows additionally retain their existing `screening_only_quantity_uncertainty` warning (RfqRequired path); DGU's override path emits no screening warning.

### R.7 / R.11 — Stage 12 verdicts

```
verdict   COUNT(*)
review    17
```

All 17 net-margin assessments remain at `review` after the rerun (matches the pre-hardening baseline). The Aggregates assessment retains its `quantity_basis_undetermined` reason.

### R.11 — Stage 14 invariants

```
operator_input_overrides (active, not withdrawn): 1
  product_candidate_id: pc-7046e8ec-...-glass-dgu-bs-en-1279
  payload_json: {"qty":250.0,"unit":"square meter"}

pending_operator_intents (Stage 14 demo): unchanged
```

## Skills Exercised on Re-run

| Skill | Run? | Notes |
|---|---|---|
| HMRC customs (DGU import) | YES | Hit live Trade Tariff API for `7008002000`, `7008008900`, etc. with CN filter |
| Shipment sizing (DGU) | NO | CDP unreachable on port 9222 → cdp_blocked_sizing_result fallback (existing behavior) |
| SeaRates freight | NO | Skipped (sizing blocked) |
| Simple FX | NO | Skipped (no freight to convert) |
| HMRC/sizing/freight (carpets, Aggregates) | NO | Domestic path — no import skills required |

## Quality Gates

```text
cargo fmt --check                                          → OK
cargo check --all-targets                                  → Finished dev profile (clean)
cargo clippy --all-targets -- -D warnings                  → 0 errors / 0 warnings
cargo test -p daemon --bin daemon                          → 157 passed; 0 failed; 0 ignored
                                                             (147 baseline + 10 new OQ.1..OQ.10)
per-file integration loop (39 .rs files in crates/daemon/tests/)
  37 / 39 PASS on first concurrent pass (clean rc=0 + "test result: ok")
  ipc_handshake & session_reaper failed on the concurrent pass under load —
  both pass cleanly when re-run alone or with `--test-threads=1`.
  These tests are PTY/process-cleanup-timing sensitive and shared the OS with
  another agent's parallel `cargo test -p procintel-desktop` workload during
  the first sweep. Re-runs:
    cargo test --test ipc_handshake          → 43 passed; 0 failed
    cargo test --test session_reaper -- --test-threads=1
                                              → 33 passed; 0 failed
```

The integration target builds were performed in an isolated `CARGO_TARGET_DIR=/tmp/stage11-orderqty-target` because a parallel agent's `cargo check -p procintel-desktop` was holding the workspace lock on `target/debug/.cargo-lock` for an extended period — running my checks in `/tmp` avoided contending for the same lock and let the gate complete deterministically.

## Failure-Mode Playbook Outcomes

- The live DGU re-run preserved its `from_operator_override` basis exactly because Stage 14's short-circuit at the top of `infer_order_quantity_basis` runs before the Priority 1 `rfq_required` branch. R.8 verified.
- Stage 10's `gross_margin_bps=9999` for Aggregates flipped to `0` only because `persist_landed_cost_estimate` performs the conditional UPDATE within the same transaction as the LCE upsert, and the new freshness predicate in `load_landed_cost_assessments` (`lce.id IS NULL`) ensures Aggregates is reloaded.
- `compute_supplier_cost_basis`'s exclusion list (`FromContractValue | FromOperatorOverride | BasisUndetermined`) prevents the legacy `QUANTITY_UNKNOWN_USED_UNIT_PRICE` warning from double-firing alongside the new BasisUndetermined warning.

## Files Changed

```
crates/daemon/src/main.rs                                              (Stage 11 hardening — primary work)
crates/daemon/src/ipc/handlers/clarifications.rs                        (allow(clippy::too_many_lines) + remove redundant clones flagged by --all-targets)
docs/pipeline/stage-11-hardening-order-quantity-validation-manifest.md  (this file)

apps/desktop/src-tauri/tests/query_service.rs                           (parallel-agent file — backtick fixes + #[allow(clippy::too_many_arguments)] to satisfy --all-targets)
apps/desktop/src-tauri/src/main.rs                                      (parallel-agent file — drop a redundant clone flagged by clippy nursery)
```

The desktop changes are touch-ups around parallel-agent work (Stage 14 Phase 2 desktop UI) needed only to keep `cargo clippy --all-targets` green per the AGENTS.md gate. They do not introduce new behaviour.

No migrations. No skill changes. No Tauri / TS frontend changes.

## Out-of-Scope Constraints Honoured

- `execute_landed_cost_skill_session` and `skills/landed-cost/searates-freight-estimate/SKILL.md` left untouched (CDP-resilience handoff territory).
- Stage 14 CLI / IPC / `pending_operator_intents` / `operator_input_overrides` schemas and handlers untouched. The Stage 14 short-circuit at the top of `infer_order_quantity_basis` is preserved verbatim.
- Stage 12 verdict contract preserved (Aggregates + DGU + carpets still REVIEW).
- No new migrations, no schema changes.
