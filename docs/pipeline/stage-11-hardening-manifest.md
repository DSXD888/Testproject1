# Stage 11 Hardening Manifest

> Generated: 2026-04-30T21:16:00Z
> Status: GREEN

## Summary

Stage 11 hardening converted the first DGU real-data success from a one-row happy path into a repeatable path with regression coverage. The daemon now preserves simple FX session/job ids, sends an explicit inferred order quantity basis to shipment sizing, persists that basis in `confidence_breakdown`, covers the new bounded-run recovery and parser policies with unit tests, and enforces deterministic CDP captcha fallback policy.

## Issue Matrix

| Issue | Status | Tests added | Behavior verified | Remaining risk |
|---|---|---|---|---|
| 1. FX skill session id wiring | GREEN | `fx_session_ids_persist_when_freight_fx_converted` | `confidence_breakdown.skill_session_ids.simple_fx` and `simple_fx_job_id` persist when FX runs. DGU rerun recorded `session-pty-b1df0973410f8bf4621793c100cf96c77f2a9d60f7e548fe0c6d172fcc3080ba` and `job-stage11-landed_cost_simple_fx-00926ab4f35245878df3d5d3655ef630`. | None known. |
| 2. Order quantity inference | GREEN | `infer_order_quantity_basis_*`, `dgu_rfq_quantity_basis_uses_moq_in_shipment_input` | Contract value, lot quantity, MOQ/RFQ fallback, and supplier-capacity fallback are deterministic. DGU now sizes from `rfq_required` qty 10 sqm, not Alibaba `quoted_quantity=10000`. | RFQ fallback remains screening-only until an operator supplies the real order quantity. |
| 3. Regression coverage for Stage 11 code paths | GREEN | Claim-by-id FIFO/negative tests; Stage11Issue default tests; agent-message parser accept/reject tests; landed-cost timeout/protocol recovery tests | `cargo test -p daemon --bin daemon` now runs 81 tests and covers the previously untested claim, defaulting, parser, timeout recovery, and protocol recovery behaviors. | None known. |
| 4. CDP captcha resilience | GREEN | `captcha_interception_without_deterministic_fallback_blocks_review`, `captcha_interception_with_matching_cached_fallback_can_review` | Captcha warnings no longer pass silently. Review is allowed only when same-URL cached/indexed evidence is explicitly marked as deterministic fallback; otherwise the daemon adds `CDP_CAPTCHA_BLOCKED_NO_DETERMINISTIC_FALLBACK`. | No captcha solver or human-wait flow is implemented; that remains out of scope for L1-L3 MVP. |

## Re-run On The DGU

Command:

```bash
cargo run -p daemon -- --ingest landed-cost
```

Daemon log: `/tmp/stage11_hardening_landed_cost_rerun.log`

Final log line:

```text
landed-cost live loop completed: attempted=1 completed=1 blocked=0 domestic=0 import=1 customs_skill_runs=1 sizing_skill_runs=1 searates_skill_runs=1 fx_skill_runs=1
```

Persisted DGU row:

| Field | Value |
|---|---|
| decision_grade | review |
| declarable_commodity_code_10 | 7008002000 |
| commodity_confidence_bps | 8400 |
| customs_resolution_confidence_bps | 8400 |
| order_quantity_basis | `rfq_required`, qty 10.0 square meter |
| shipping_mode | LCL |
| supplier_cost_gbp_pence | 5160 |
| freight_low / base / high | 17692 / 17692 / 20801 |
| customs_duty_gbp_pence | 0 |
| trade_remedy_duty_gbp_pence | 0 |
| import_vat_gbp_pence | 4570 |
| economic_landed_cost_gbp_pence | 27422 |
| net_margin_low / base / high | 0 / 0 / 0 |
| blockers | `[]` |

Arithmetic check: computed 27422, stored 27422, delta 0.

Cohort health: 4 landed-cost rows, all `review`; the 3 domestic rows remain `review` with zero freight, zero duty/VAT/fees, and `economic_landed_cost_gbp_pence == supplier_cost_gbp_pence`.

## Quality Gates

```text
cargo fmt --check                         PASS
cargo check --all-targets                 PASS
cargo clippy --all-targets -- -D warnings PASS
cargo test -p daemon --bin daemon         PASS - 81 tests
```

## Data Snapshots

- Before: `/tmp/landed_cost_before_stage11_hardening.sql`
- After: `/tmp/dgu_landed_cost_after_stage11_hardening.txt`
- Daemon log: `/tmp/stage11_hardening_landed_cost_rerun.log`

## Out-of-Scope

- No follow-up bead was required for failing tests; the full daemon bin test suite passed.
- Captcha solving and manual operator wait are intentionally not implemented in this hardening pass.
