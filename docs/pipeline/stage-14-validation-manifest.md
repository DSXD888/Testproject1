# Stage 14 Operator Decision (Phase 1 — CLI) Validation Manifest

> Generated: 2026-05-01T20:36Z
> Status: GREEN

## Summary

Stage 14 Phase 1 ships the operator-decision CLI capture path end-to-end. The §3 demo walked the DGU process from clarification capture (`quantity_basis_unaligned`, qty=100 m²) through pipeline re-run, manifest re-render with a fresh `manifest_basis_hash`, decision capture (`approve` then `watch`), idempotency no-op, audit-chain supersede, stale detection (after a second clarification at qty=250 produced a third manifest basis), withdrawal, and queued-list filter. The override injection in Stage 11 changed the DGU's `landed_cost_estimates.confidence_breakdown.order_quantity_basis` from `rfq_required (qty=10)` to `from_operator_override (qty=100 → qty=250)` across two pipeline reruns. The `manifest_basis_hash` cycled `b2cbfc… → c51aba… → fb75ba…`, and `pending_operator_intents` accumulated three statuses (superseded, stale, withdrawn) across two intents — exactly as designed. Stage 12 picked up 17 candidates as `verdict='review'`, all 7 manifests stayed `manifest_ready/clarify`, and zero FK orphans on the new override / intent tables.

## Build Summary

- Migration: `migrations/0009_operator_input_overrides.sql` (new file; bare DDL, no `BEGIN;`/`COMMIT;`).
- Migration registered: `crates/daemon/src/db/sqlite.rs` `MIGRATIONS` array, epoch 6, plus `verify_l3_schema` now also asserts `operator_input_overrides` exists.
- Schema activation tests:
  - Renamed/replaced epoch test: `pdf_t_schema_activation_009_default_schema_epoch_is_5` → `_is_6` (asserts `DEFAULT_SCHEMA_EPOCH == 6` AND `user_version == 6`).
  - New: `opd_t_schema_activation_009_upgrade_path_applies_0009_cleanly` (replays migrations 0001-0008, opens StateDb, asserts override table + columns + both indexes + bumped user_version).
- New CLI subcommands on `Cli`: `--capture-decision`, `--disposition`, `--idempotency-key`, `--reasoning`, `--capture-clarification`, `--candidate-id`, `--reason-code`, `--payload`, `--note`, `--withdraw-clarification`, `--withdraw-decision`, `--list-decisions`, `--process-id`, `--list-status-filter`. Dispatched in `dispatch_operator_action(...)`, called from `main()` BEFORE the `--ingest` match.
- Override-injection points:
  - **Stage 11**: `LandedCostAssessmentRow.operator_override: Option<OperatorQuantityOverride>` populated by `load_landed_cost_assessments` via `read_active_quantity_override`. `infer_order_quantity_basis` short-circuits to `OrderQuantityBasis::FromOperatorOverride { qty, basis_unit }`. New variant added to `kind/source/supplier_cost_basis/quantity/unit/to_json/screening_uncertainty_warning` impls. `compute_supplier_cost_basis` short-circuits when the override variant is active. `JSON kind` emitted as `"from_operator_override"`.
  - **Stage 11 freshness**: the loader's WHERE clause now includes `OR EXISTS (... operator_input_overrides ... AND oio.updated_at > lce.updated_at)` so the live-loop reprocesses landed-cost rows when an override is freshly captured or withdrawn.
  - **Stage 10 RFQ**: `allocate_gross_margin_revenue` now reads `read_active_rfq_override` first; on hit, returns `RevenueAllocation { revenue_basis_type: "expected_value", revenue_pence = unit_price × qty, confidence_bps: 8000 }` where qty comes from a quantity-override or RFQ `moq` fallback.
  - **`no_pricing_signal`**: recorded only — no auto-synthesis. Confirmed by T.12.
- Stale detection: `mark_intents_stale_on_basis_drift(tx, process_id, new_manifest_basis_hash)` called inside `persist_manifest_generation_output` (the shared persistence helper used by all 3 Stage 13 render modes), inside the same `with_write_transaction`.
- New unit tests: T.1–T.18 (all passing). Test count went 129 → 147 binary unit tests; schema_activation file gained the OPD-009 test.

## End-to-End Demo Output (per §3)

Captured to `/tmp/stage14_demo.log` and `/tmp/stage14_R_validation.log`. Highlights:

```
=== STEP 1: Baseline ===
manifest_basis_hash:   b2cbfc0adf23d1da0875feeab65cff7b26a46d3f74aae0ed3b8d3d7b0d119f49
DGU lce basis_kind:    rfq_required (qty=10.0)
user_version:          5

=== STEP 1b: daemon --dry-run applies migration 0009 ===
user_version (post):   6
operator_input_overrides table created.

=== STEP 2: --capture-clarification (qty=100, square meter) ===
{
  "basis_hash_when_provided": "b2cbfc0adf23d1da0875feeab65cff7b26a46d3f74aae0ed3b8d3d7b0d119f49",
  "candidate_id": "...glass-dgu-bs-en-1279",
  "payload": {"qty":100.0,"unit":"square meter"},
  "reason_code": "quantity_basis_unaligned",
  "stage_run_id": "stage-run-0c3d9868-..."
}

=== STEP 3a-c: --ingest landed-cost / net-gate / manifest ===
landed-cost: attempted=1 customs_skill_runs=1 (CDP/SeaRates blocked → expected in this env)
net-gate:    attempted=17 candidates_assessed=1 review=1 skipped_unchanged=16
manifest:    attempted=7 processes_rendered=1 stoplight_clarify=1 skipped_unchanged=6

=== STEP 4: New manifest_basis_hash ===
c51aba0875297fc95625aa020c2d46bb78a74849c338d8f8086e64e8fdf4e93c   (changed)

=== STEP 5: --capture-decision (approve, dec-2026-05-01-dgu-001) ===
{
  "intent_id": "intent-5045f346fb0246b7a23d9eda308ff073",
  "disposition": "approve",
  "status": "queued",
  "reviewed_basis_hash": "4e7916264...",
  "reviewed_manifest_hash": "581bf200b...",
  "reviewed_manifest_basis_hash": "c51aba087...",
  "idempotent_no_op": false,
  "superseded_id": null
}

=== STEP 7: Idempotency replay returns the same intent_id with idempotent_no_op=true ===
COUNT(*) WHERE idempotency_key='dec-2026-05-01-dgu-001' → 1

=== STEP 8: --capture-decision (watch, dec-2026-05-01-dgu-002) supersedes 001 ===
dec-001 → status=superseded, superseded_by_intent_id=intent-a33a4a3f23df45838c1d5d640e440db4
dec-002 → status=queued

=== STEP 9: Capture qty=250 override → re-run pipeline ===
landed-cost: lce updated, basis_kind=from_operator_override qty=250.0
manifest_basis_hash: fb75ba219f0a42a8ef5ae58c5d1753a08887fe20d785319f0386196577df7b25 (3rd hash)
Stale detection fired: dec-002 (queued) → stale (reviewed_basis_hash=4e7916264... ≠ new manifest's basis driver)

=== STEP 10: --withdraw-decision dec-2026-05-01-dgu-002 ===
{
  "previous_status": "stale",
  "previous_disposition": "watch",
  "status": "withdrawn",
  ...
}

=== STEP 11: --list-decisions --list-status-filter queued ===
{ "decisions": [], ... }
```

## R.1–R.16 Acceptance Criteria

| #    | Criterion                                                                                         | Result | Detail                                                                                                                                                                                  |
| ---- | ------------------------------------------------------------------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R.1  | Migration 0009 + epoch bump to 6 + epoch test renamed                                             | PASS   | `MIGRATIONS` includes `("0009_operator_input_overrides", 6, ...)`. `DEFAULT_SCHEMA_EPOCH = 6`. `pdf_t_schema_activation_009_default_schema_epoch_is_6` asserts both code constant and `user_version`. |
| R.2  | `operator_input_overrides` table with all expected columns + UNIQUE + indexes                     | PASS   | Column set matches §2.1 (17 columns + `UNIQUE(process_id, product_candidate_id, reason_code)`). Both indexes present (`idx_oio_active`, `idx_oio_process`).                              |
| R.3  | New schema activation test exists and passes                                                     | PASS   | `opd_t_schema_activation_009_upgrade_path_applies_0009_cleanly` passes in the per-file integration loop.                                                                                 |
| R.4  | CLI subcommands work end-to-end per §3 demo                                                       | PASS   | All 11 demo steps captured to `/tmp/stage14_demo.log`. JSON output emitted by every operator action.                                                                                     |
| R.5  | 3-state disposition enforcement                                                                   | PASS   | T.2 (`capture_decision_rejects_invalid_disposition`) passes; CLI returns clear `invalid disposition 'hold'` error before any DB write.                                                  |
| R.6  | Idempotency: duplicate `--idempotency-key` is no-op-with-success                                  | PASS   | T.3 + demo step 7 confirm: 2 invocations, 1 row, `idempotent_no_op: true` returned on second call.                                                                                       |
| R.7  | Re-decision audit chain                                                                           | PASS   | T.4 + demo step 8 confirm: `dec-001` → `status='superseded'`, `superseded_by_intent_id` points to `dec-002`'s intent_id.                                                                 |
| R.8  | Override persistence: 1 active row per (proc, candidate, reason)                                  | PASS   | T.7 + R.8 query (rows-having-active>1 must be empty) returned no rows.                                                                                                                  |
| R.9  | Stage 11 `infer_order_quantity_basis` returns `FromOperatorOverride`                              | PASS   | T.9 (pure-fn) + R.9 query: `confidence_breakdown.order_quantity_basis.kind` = `from_operator_override`, qty = 250.0 after second override.                                              |
| R.10 | RFQ override flow                                                                                 | PASS   | T.11 confirms `allocate_gross_margin_revenue` short-circuits: `revenue_basis_type='expected_value'`, `revenue_pence = unit_price × qty`, `confidence_bps >= 8000`.                       |
| R.11 | `no_pricing_signal` recorded only — does not auto-synthesize                                      | PASS   | T.12 confirms 1 override row + 0 pricing-assessment rows synthesized.                                                                                                                   |
| R.12 | Stale detection on basis_hash drift                                                               | PASS   | T.14 + demo step 9: `dec-002` transitioned `queued → stale` after Stage 13 re-render with fresh `manifest_basis_hash` (b2cbfc → c51aba → fb75ba).                                       |
| R.13 | Stage tracking emits `'operator decision' / process / succeeded`                                  | PASS   | `process_stage_status`: 5 succeeded rows (one per state-mutating CLI invocation). `process_stage_heads`: 1 row.                                                                          |
| R.14 | FK integrity: 0 orphans on overrides/intents and dangling supersede references                    | PASS   | All 5 R.14 sub-queries returned 0.                                                                                                                                                      |
| R.15 | Stage 11/12/13 invariants intact                                                                  | PASS   | 17 `candidate_net_margin_assessments` (review), 7 `pipeline_assessments` (manifest_ready), 7 `manifests` (clarify). DGU lce specifically updated to qty=250 — that is the expected change.|
| R.16 | Quality gates                                                                                     | PASS   | fmt ✓, check ✓, clippy `-D warnings` ✓, 147 binary unit tests ✓, 39/39 integration test files ✓.                                                                                          |

## Quality Gates

- `cargo fmt --check`: PASS
- `cargo check --all-targets`: PASS
- `cargo clippy --all-targets -- -D warnings`: PASS
- `cargo test -p daemon --bin daemon`: PASS — **147 tests passed (was 129; T.1–T.18 added)**
- Per-file integration loop: PASS — 39/39 (`schema_activation.rs` now contains 22 tests including new OPD-009)

## Issues Found & Fixed

1. **Move-after-borrow in `capture_operator_decision`**: the `with_write_transaction` closure captured `snapshot` by move, leaving the post-transaction `json!({...})` unable to read `snapshot.decision_basis_hash` / `artifact_hash`. Fix: clone both fields into `_for_query` locals before the closure.
2. **Loader staleness**: the original `load_landed_cost_assessments` WHERE clause only refreshed lce rows on `quote_row_version` drift or non-empty blockers, so a fresh override would not trigger reprocessing. Fix: added `OR EXISTS (... operator_input_overrides ... AND oio.updated_at > lce.updated_at)`. Verified by step 3a/9 — both pipeline reruns picked up the DGU candidate (attempted=1).
3. **Clippy `too_many_lines` / `struct_field_names` / `redundant_clone` / `useless_conversion` / `similar_names`**: all addressed inline (added `#[allow]` for the four legitimately long handler functions; renamed `cli_a1/a2/b1` to `cli_alpha/beta/gamma`; dropped redundant clones; collapsed `Ok(...?)`).
4. **Migration epoch test rename**: anticipated by handoff §0; renamed `..._is_5` → `..._is_6` to match the new `DEFAULT_SCHEMA_EPOCH = 6`. Same gotcha as Stages 12 and 13.

## Code Changes

| File                                                | Change                                                                                                                                                                       |
| --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `migrations/0009_operator_input_overrides.sql`      | New: `operator_input_overrides` table + `idx_oio_active` (partial WHERE active) + `idx_oio_process`. Bare DDL — `apply_sql_file` already wraps in SAVEPOINT.                  |
| `crates/daemon/src/db/sqlite.rs`                    | New `L_OPERATOR_OVERRIDES_MIGRATION_PATH`, new `MIGRATIONS` entry at epoch 6, `DEFAULT_SCHEMA_EPOCH = 6`, `verify_l3_schema` adds `operator_input_overrides` check.            |
| `crates/daemon/tests/schema_activation.rs`          | Renamed `..._epoch_is_5` → `..._epoch_is_6` (asserts user_version == 6 + DEFAULT_SCHEMA_EPOCH == 6). New `opd_t_schema_activation_009_*` test (replay migrations + assertions). |
| `crates/daemon/src/main.rs`                         | `OrderQuantityBasis::FromOperatorOverride` variant + impls; `LandedCostAssessmentRow.operator_override`; `infer_order_quantity_basis` short-circuit; `compute_supplier_cost_basis` short-circuit; new types `OperatorOverrideReason`, `OperatorQuantityOverride`, `OperatorRfqOverride`; new helpers `validate_operator_override_payload`, `read_active_quantity_override`, `read_active_rfq_override`; loader-WHERE freshness clause; `allocate_gross_margin_revenue` RFQ short-circuit; `Cli` 12 new fields; `dispatch_operator_action`, `capture_operator_decision`, `capture_operator_clarification`, `withdraw_operator_clarification`, `withdraw_operator_decision`, `list_operator_decisions`, `read_manifest_snapshot_for_decision`, `read_latest_manifest_basis_hash`, `lookup_intent_by_idempotency_key`, `mark_intents_stale_on_basis_drift`, `OPERATOR_DECISION_STAGE_NAME`, `OPERATOR_DECISION_DISPOSITIONS`, `OPERATOR_DECISION_STATUSES`, `DEFAULT_LOCAL_OPERATOR_ID`. `persist_manifest_generation_output` calls `mark_intents_stale_on_basis_drift` inside its existing transaction. 18 new tests T.1–T.18 in `mod tests`. |

## Out-of-Scope (Documented, Not Fixed Here)

- Phase 2 Tauri desktop UI (separate handoff). `apps/desktop/` untouched.
- 4-state disposition with `hold` (would require CHECK-rewrite migration; out of scope).
- Non-GBP RFQ override currency conversion (Phase 1 rejects non-GBP RFQ payloads with a clear error).
- Auto-synthesis of `candidate_pricing_assessments` from `manual_price` overrides (deferred to a follow-up bead — Stage 10's flow is built around `supplier_quotes` and synthetic injection is non-trivial; recording the override Phase-1 keeps the operator action durable).
- Bid workflow execution (Stage 15 — separate stage).
- Remote `operator_decision_heads` table + CAS sync (L4 — separate work).
- The current live `state.db` shows DGU's lce with non-empty `blockers` (CDP_CONNECT_FAILED, SEARATES_NO_RESULT) because the validation environment had no running CDP browser / SeaRates skill. This is purely environmental — the override-injection path is functional (basis_kind = `from_operator_override`, qty = 250) and the manifest/decision flow is demonstrated; clearing the freight/customs blockers is unrelated to Stage 14.
