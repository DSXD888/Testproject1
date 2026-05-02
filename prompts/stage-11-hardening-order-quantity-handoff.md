# Stage 11 Hardening — Order-Quantity Inference for Non-RFQ Candidates

ULTRATHINK and be meticulous.

## MISSION

Stage 11's `infer_order_quantity_basis` (at `crates/daemon/src/main.rs:3821`) currently has 4 production variants: `FromContractValue`, `FromLotQuantity`, `RfqRequired`, `SupplierCapacity` (plus `FromOperatorOverride` added in Stage 14). For non-RFQ `expected_value` candidates without an operator override, the inference can produce **operationally meaningless margins** when revenue and supplier-cost are on different quantity bases. The Aggregates row is the canonical example: revenue = framework total over multi-year period (£167k), supplier_cost = price-per-pack from amazon-uk (£15.40), inferred basis falls through to a high-margin computation that's mathematically real but operationally wrong.

Stage 14 added the **operator override** mechanism as the proper long-term fix (operator provides actual order qty, daemon honors it). This handoff is about **everything Stage 11 can do BEFORE the operator overrides** — better defaults, better confidence calibration, better warnings, so that the manifest never claims "9999 bps margin" when the basis is genuinely uncertain.

You will **build → validate → test → fix in a loop** until every numbered acceptance criterion (R.1–R.12 below) is GREEN. Specifically: the Aggregates row's net margin gate verdict must continue to be REVIEW (it already is via Stage 12's `quantity_basis_undetermined` check), but its underlying confidence and warning surface should be **honest about the basis problem** instead of hiding it.

```
┌────────────────────────────────────────────────────────────┐
│                          THE LOOP                            │
│   1. Pre-flight (§0)                                         │
│   2. Implement: confidence calibration + warning surface +   │
│      improved heuristics for expected_value-no-basis case   │
│   3. Re-run --ingest landed-cost (forces Stage 11 recompute)│
│   4. Validate against R.1..R.12 (queries inline)             │
│   5. Failures? → diagnose, fix, repeat from step 3           │
│   6. Quality gates: full test suite per AGENTS.md            │
│   7. All green → write the validation manifest               │
└────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first.

---

## §0 — PRE-FLIGHT

| Claim | Verify |
|---|---|
| `infer_order_quantity_basis(row: &LandedCostAssessmentRow) -> OrderQuantityBasis` at `main.rs:3821` is the central inference. Stage 14 added `FromOperatorOverride` short-circuit at the top — this handoff modifies the FALLBACK branches BELOW that short-circuit | `grep -nB2 -A60 "fn infer_order_quantity_basis" crates/daemon/src/main.rs` |
| `OrderQuantityBasis::kind()` and `::source()` and `::screening_uncertainty_warning()` impls live near the enum definition (search the whole file — Stage 14 added impls in multiple places). New variants would extend them | `grep -n "impl OrderQuantityBasis\|fn kind\|fn source\|fn screening_uncertainty_warning" crates/daemon/src/main.rs` |
| `LandedCostAssessmentRow` has `revenue_basis_type`, `expected_addressable_revenue_ex_vat_pence`, `quoted_quantity`, `quoted_quantity_unit`, `minimum_order_quantity`, `unit_price_gbp_pence`, `quote_cost_boundary` (and from supplier_quotes: `price_basis_qty`, `price_basis_unit`, `listing_pack_qty`, `listing_pack_unit`) | `grep -A35 "^struct LandedCostAssessmentRow" crates/daemon/src/main.rs` |
| Stage 12 D5 quantity-basis-mismatch check at `main.rs` (in the net-margin verdict logic) catches `quantity_basis_undetermined` cases and forces REVIEW. **Stage 11 hardening must not break this contract** — current Aggregates lands as REVIEW, that must remain | `grep -n "quantity_basis_undetermined\|quantity_basis_unaligned" crates/daemon/src/main.rs \| head -10` |
| Aggregates' live data: revenue=£167,222 (`expected_value`), supplier quote raw price=88p, price_basis_qty=17.5, price_basis_unit=kg. **CRITICAL** (verified 2026-04-30): Aggregates' `confidence_breakdown` JSON does NOT have an `order_quantity_basis` key today — only `supplier_cost_basis` is emitted. The `order_quantity_basis` key is currently emitted ONLY for the Stage 14 override path (DGU has it, Aggregates and carpets do not). To make R.6's validation query meaningful, **this handoff must extend Stage 11 persistence to ALWAYS emit `order_quantity_basis` into `confidence_breakdown`** (override path + non-override path both populate it) | `sqlite3 state/state.db "SELECT json_extract(confidence_breakdown, '\$.order_quantity_basis'), json_extract(confidence_breakdown, '\$.supplier_cost_basis') FROM landed_cost_estimates WHERE product_candidate_id LIKE '%aggregates-general%'"` |
| Live baseline (verified 2026-04-30): Aggregates `gross_margin_bps=9999, commercial_estimate_confidence_bps=4000`. Carpets `gross_margin_bps=0, commercial_estimate_confidence_bps=1000` (i.e. the D5 calibration is already applied to RfqRequired-without-override path — only `BasisUndetermined` is genuinely new behavior). DGU `from_operator_override, gross=0, comm_conf=1000` (Stage 14 override active) | `sqlite3 state/state.db "SELECT pc.name, json_extract(lce.confidence_breakdown, '\$.order_quantity_basis.kind') AS k, lce.commercial_estimate_confidence_bps, cpa.gross_margin_bps FROM landed_cost_estimates lce JOIN candidate_pricing_assessments cpa ON cpa.product_candidate_id=lce.product_candidate_id AND cpa.selected_quote_id=lce.supplier_quote_id JOIN product_candidates pc ON pc.id=lce.product_candidate_id ORDER BY pc.name"` |
| `notice_lots.estimated_value_pence` (lot-level value, distinct from process-level contract value) is available — could be used to infer per-lot quantity rather than per-process | `sqlite3 state/state.db "SELECT id, lot_key, estimated_value_pence FROM notice_lots WHERE id LIKE '%cpu8309%' OR title LIKE '%aggregate%' LIMIT 5"` |
| `agent-mail` healthy; commits need `AGENT_NAME=claude-code git commit ...` | inline at commit time |

**Parallel-safety note**: this handoff modifies `crates/daemon/src/main.rs` in the Stage 11 area (~line 3821 for `infer_order_quantity_basis`; the impl blocks and `load_landed_cost_assessments` (~4196) plus the `build_landed_cost_estimate` callees may also need touches). The CDP captcha resilience handoff (running in parallel) modifies `execute_landed_cost_skill_session` (~line 4497) and `skills/landed-cost/searates-freight-estimate/SKILL.md`. No overlap, but use `git worktree add ../testproject1-stage11-orderqty main` for clean isolation. Merge to main with rebase at the end. **Do NOT touch `execute_landed_cost_skill_session`** or skill prompts — that's the captcha handoff's territory.

---

## §1 — DECISIONS ALREADY MADE

### D1 — Honest confidence over hidden faulty math

The current behavior produces a 9999 bps gross margin for Aggregates because revenue (£167k) divided by supplier_cost (£15.40) gives ~99.99%. Stage 12 D5 catches this and forces REVIEW with `quantity_basis_undetermined`. **The verdict is correct but the underlying numbers are still misleading** — anyone querying `gross_margin_bps` or `net_margin_bps` directly would see a 9999 and assume good signal.

The fix: when `infer_order_quantity_basis` returns a low-confidence basis (RfqRequired with no override, or a new "BasisUndetermined" variant), Stage 11 should **persist gross/net margin as 0 (or NULL) AND set the confidence to a calibrated low value**, not the inflated 9999. This makes the bad-basis case visible at every layer, not just at Stage 12's verdict step.

### D2 — New explicit variant: `BasisUndetermined`

Add a 6th variant to `OrderQuantityBasis`:

```rust
BasisUndetermined {
    reason: BasisUndeterminedReason,        // enum: revenue_cost_scale_mismatch | no_quantity_anywhere | conflicting_signals
    note: String,                           // human-readable explanation
}
```

Distinct from `RfqRequired` (which means "operator must supply RFQ") and `SupplierCapacity` (which means "we have a quoted_quantity but it's the supplier's tier, not the buyer's order"). `BasisUndetermined` means: **we cannot infer ANY meaningful order quantity from the available data; downstream stages must treat margins as 0 / unknown until clarified**.

When this variant is returned:
- `OrderQuantityBasis::kind()` → `"basis_undetermined"`
- `OrderQuantityBasis::source()` → `"insufficient_inputs"`
- `OrderQuantityBasis::screening_uncertainty_warning()` → returns a Stage11Issue warning code `QUANTITY_BASIS_UNDETERMINED_NO_OPERATOR_OVERRIDE`
- `OrderQuantityBasis::quantity()` → returns `None` (or 0.0 — pick the cleaner shape; consistency with existing variants matters)

### D3 — Inference logic refinement

The inference order (after `FromOperatorOverride` short-circuit) should be:

| Priority | Rule | Variant |
|---|---|---|
| 1 | If `revenue_basis_type='rfq_required'` | `RfqRequired { fallback_qty, basis_unit }` (existing) |
| 2 | If `revenue_basis_type='expected_value'` AND `quoted_quantity` is set AND looks plausible (see D4) | `FromContractValue { qty, contract_value, basis_unit }` (existing — clarified) |
| 3 | If lot has `estimated_value_pence` set AND a sensible per-unit derivation works | `FromLotQuantity { qty, basis_unit }` (existing — extended to actually use `notice_lots.estimated_value_pence` if not already) |
| 4 | If `quoted_quantity` is set but suspect (D4 plausibility check fails) | `SupplierCapacity { qty, basis_unit }` + warning |
| **5 (NEW)** | **Otherwise: nothing else gives a usable basis** | **`BasisUndetermined { reason, note }`** |

The current code falls through to one of the existing 4 in the catch-all case; this handoff replaces that fallthrough with `BasisUndetermined`.

### D4 — Plausibility check for `quoted_quantity`

`quoted_quantity` is the quantity field on `supplier_quotes`. For Alibaba listings it's often the **supplier's tier capacity** (10000 m² for DGU), not a buyer order. Heuristic to detect:

- If `quoted_quantity > 1000` AND `quoted_quantity_unit` is a bulk unit (m², kg, piece, etc.) AND `minimum_order_quantity` is much smaller (or 1) → likely supplier capacity, not order qty
- If `quoted_quantity` matches `listing_pack_qty` exactly → it's a pack, not an order
- If `quoted_quantity * unit_price_gbp_pence` exceeds `expected_addressable_revenue_ex_vat_pence * 10` → cost basis exceeds revenue scope by 10×, likely scale mismatch

When suspect, prefer `SupplierCapacity` variant (with appropriate warning) OR `BasisUndetermined`.

### D5 — Confidence calibration in Stage 11 output

**Note**: the `RfqRequired-without-override` calibration is ALREADY in place today (carpets are at gross=0, comm_conf=1000 in the live DB — verified 2026-04-30). What's new in this handoff is applying the SAME calibration to the new `BasisUndetermined` variant.

When the basis is `BasisUndetermined` (or remains `RfqRequired` without override):

- `commodity_confidence_bps`, `customs_resolution_confidence_bps` → unchanged (these are about HMRC, not quantity)
- `commercial_estimate_confidence_bps` → set to **1000 (10%)** — explicitly low because the cost basis is undetermined
- `gross_margin_bps`, `net_margin_low/base/high_bps` → set to **0** (not the inflated 9999)
- `customs_cost_spread_bps` → unchanged
- `economic_landed_cost_gbp_pence` → still computed (it's the per-unit landed cost; that's still meaningful even without order qty)

**Aggregates** is the test case for this calibration extension — currently gross=9999, comm_conf=4000; after this handoff, gross=0, comm_conf=1000.

Stage 12 sees: gross_margin=0 + commercial_confidence=1000 → REVIEW with the same `quantity_basis_undetermined` reason it already emits. Net effect: **same Stage 12 verdict for Aggregates, but honest underlying numbers**.

### D5b — `order_quantity_basis` JSON field must be emitted for ALL rows

Today (verified 2026-04-30), `confidence_breakdown.order_quantity_basis` is populated ONLY for the `from_operator_override` path. Aggregates and carpets have no such key (they only have `supplier_cost_basis`). For R.6's validation query to work, **Stage 11 persistence must be extended to emit `order_quantity_basis: {kind, source, qty?, unit?, reason?, note?}` for every variant — not only override**.

Add a small serialiser (e.g., `OrderQuantityBasis::to_json_value()`) that maps each variant to a stable JSON shape, then call it in the place that builds `confidence_breakdown` (look for where `supplier_cost_basis` is currently emitted — likely in `build_landed_cost_estimate`).

### D6 — Backwards-compat: existing rows

The 4 existing `landed_cost_estimates` rows were computed before this handoff. After the change, they'll be reprocessed on the next `--ingest landed-cost` run (Stage 14's loader-freshness clause already triggers refresh on basis changes; you can also explicitly delete and re-run). Expect:

- Aggregates row: gross_margin_bps changes from 9999 → 0 (or a lower honest number); commercial_confidence drops; basis_kind becomes `basis_undetermined` if no override is active, OR `from_operator_override` if the Stage 14 demo's qty=250 override is still active. Verify which case applies in your live DB before predicting the post-run state.
- Other rows: should be unaffected unless they had similar basis problems.

---

## §2 — IMPLEMENTATION ORDER

1. Add `BasisUndetermined` variant + `BasisUndeterminedReason` enum.
2. Update `OrderQuantityBasis::kind()`, `::source()`, `::screening_uncertainty_warning()`, `::quantity()`, `::unit()`, `::supplier_cost_basis()` impls and any `match` arms (compile-error-driven; let `cargo check` enumerate them).
3. Implement the D4 plausibility check helper.
4. Refactor `infer_order_quantity_basis` to follow D3 priority order, returning `BasisUndetermined` in the catch-all.
5. Modify Stage 11 persistence to apply D5 confidence calibration when basis is `BasisUndetermined` or `RfqRequired`.
6. Add unit tests (§4).
7. Re-run `--ingest landed-cost` against the live DB. Inspect Aggregates' row.
8. Verify Stage 12 verdict for Aggregates is still REVIEW with `quantity_basis_undetermined` (or `_unaligned` per existing logic).
9. Run quality gates.
10. Write the validation manifest.

---

## §3 — EXPECTED RESULTS

After the work + a fresh `--ingest landed-cost` run with **no Stage 14 operator override active for Aggregates** (delete the operator_input_overrides row for Aggregates first if one exists from any demo activity):

| Aggregates field | Before (verified 2026-04-30) | After |
|---|---|---|
| `confidence_breakdown.order_quantity_basis.kind` | NULL (key not emitted today) | `basis_undetermined` (after D5b serializer added) |
| `confidence_breakdown.order_quantity_basis.reason` | NULL | `revenue_cost_scale_mismatch` or `no_quantity_anywhere` |
| `gross_margin_bps` (in pricing_assessments) | 9999 | 0 |
| `commercial_estimate_confidence_bps` | 4000 | 1000 |
| Stage 12 verdict | review | review (unchanged) |
| Stage 12 reasons | `quantity_basis_undetermined` | `quantity_basis_undetermined` (unchanged) |

For the DGU row with the active Stage 14 override at qty=250: should remain `from_operator_override` and the override values preserved. The Stage 14 demo state must not regress.

For the 3 carpet rows (RFQ-required from ProContract): basis stays `RfqRequired`; their gross/net margins are ALREADY 0 + commercial_confidence ALREADY 1000 (the D5 calibration was applied in earlier hardening). After this handoff, the only visible change is that `order_quantity_basis: {kind: "rfq_required", source: "rfq_pending", ...}` will now appear in their `confidence_breakdown` (it's NULL today). Stage 12 still REVIEW with `awaiting_rfq_quote`.

---

## §4 — REQUIRED UNIT TESTS

Add to `crates/daemon/src/main.rs` (`mod tests`):

| # | Test | Verifies |
|---|---|---|
| OQ.1 | `infer_order_quantity_basis_returns_basis_undetermined_when_no_signals` | Synthetic row with revenue_basis_type='expected_value', no quoted_quantity, no lot value → `BasisUndetermined { reason: NoQuantityAnywhere }` |
| OQ.2 | `infer_order_quantity_basis_returns_basis_undetermined_on_revenue_cost_scale_mismatch` | Aggregates-shape: revenue=£167k, supplier_cost (qty × unit) ≪ 1% of revenue → `BasisUndetermined { reason: RevenueCostScaleMismatch }` |
| OQ.3 | `infer_order_quantity_basis_returns_supplier_capacity_when_quoted_qty_suspect` | DGU-shape (no override): quoted_quantity=10000 m², MOQ=10 → `SupplierCapacity` (existing variant; verify the heuristic chooses it over `FromContractValue`) |
| OQ.4 | `infer_order_quantity_basis_uses_lot_estimated_value_when_present` | Synthetic row where notice_lots.estimated_value_pence is set + plausible per-unit math → `FromLotQuantity` |
| OQ.5 | `infer_order_quantity_basis_operator_override_short_circuits_basis_undetermined` | Same Aggregates-shape as OQ.2 BUT with active operator_input_overrides row → `FromOperatorOverride` (Stage 14's path takes precedence; verify the new logic doesn't regress this) |
| OQ.6 | `stage11_persists_zero_margin_when_basis_undetermined` | After build_landed_cost_estimate runs on a row with `BasisUndetermined`, the persisted gross_margin_bps and net_margin_*_bps fields are 0, commercial_estimate_confidence_bps is 1000 |
| OQ.7 | `stage11_persists_zero_margin_when_rfq_required_no_override` | Same as OQ.6 but for `RfqRequired` variant |
| OQ.8 | `basis_undetermined_warning_emitted_in_warnings_array` | Stage 11 issue with code `QUANTITY_BASIS_UNDETERMINED_NO_OPERATOR_OVERRIDE` appears in the row's warnings JSON |
| OQ.9 | `kind_source_warning_helpers_return_expected_strings_for_basis_undetermined` | `BasisUndetermined.kind()=="basis_undetermined"`, `.source()=="insufficient_inputs"`, `.screening_uncertainty_warning()` returns Some with the new code |
| OQ.10 | `existing_4_variants_unchanged_after_addition` | Existing variants (FromContractValue, FromLotQuantity, RfqRequired, SupplierCapacity) still serialize/deserialize and compute correctly — regression suite |

**Test count expectation**: 10 new unit tests. Current binary unit count: 147 (post-Stage 14). After: **157**. Plus all existing 147 must still pass.

**Existing tests to verify (do NOT delete, may need updates)** — the inference function already has 4 unit tests at `crates/daemon/src/main.rs:22124-22186`:
- `infer_order_quantity_basis_prefers_contract_value` (22124)
- `infer_order_quantity_basis_uses_lot_quantity_when_value_absent` (22144)
- `infer_order_quantity_basis_uses_moq_for_rfq_required` (22161)
- `infer_order_quantity_basis_marks_supplier_capacity_as_last_resort` (22176)

After adding `BasisUndetermined`, the catch-all behavior changes. If the synthetic inputs in any of these tests previously fell through to a fallback variant and now fall through to `BasisUndetermined`, the assertions need updating. **Read each test, predict whether the input still triggers the same variant under the new D3 priority order, and update assertions if not**. Do not silently `#[ignore]` failing existing tests.

---

## §5 — ACCEPTANCE CRITERIA — R.1 to R.12

| # | Criterion |
|---|---|
| R.1 | New `BasisUndetermined { reason, note }` variant exists on `OrderQuantityBasis`; `BasisUndeterminedReason` enum has at least `RevenueCostScaleMismatch`, `NoQuantityAnywhere`, `ConflictingSignals` |
| R.2 | All `match OrderQuantityBasis` sites updated (Rust compile-error driven); `cargo check --all-targets` passes |
| R.3 | `infer_order_quantity_basis` follows D3 priority order; D4 plausibility check correctly classifies the Aggregates and DGU shapes per §3 expected results |
| R.4 | Stage 11 persistence: when basis is `BasisUndetermined` (NEW) OR `RfqRequired` without override (existing), persisted `gross_margin_bps`=0, `net_margin_low/base/high_bps`=0, `commercial_estimate_confidence_bps`=1000. **Aggregates flips from 9999/4000 → 0/1000**; carpets stay at 0/1000 (already there). |
| R.5 | A new Stage11Issue warning code `QUANTITY_BASIS_UNDETERMINED_NO_OPERATOR_OVERRIDE` is emitted into the row's `warnings_json` when basis is `BasisUndetermined` |
| R.6 | After D5b serializer added, ALL `landed_cost_estimates` rows have `confidence_breakdown.order_quantity_basis.kind` populated (not NULL). Aggregates: `kind="basis_undetermined"`. DGU: `kind="from_operator_override"`. Carpets: `kind="rfq_required"`. Verified by `json_extract` returning non-NULL on every row |
| R.7 | Stage 12 verdict for Aggregates remains REVIEW with reasons containing `quantity_basis_undetermined` (Stage 12 contract unchanged) |
| R.8 | DGU row (with Stage 14 active override at qty=250) remains `from_operator_override`; gross/net margins unchanged from Stage 14 demo state |
| R.9 | Carpet rows: basis stays `RfqRequired`; gross/net margins remain 0, commercial_confidence remains 1000 (already true today — only the new `order_quantity_basis` JSON key is added) |
| R.10 | OQ.1–OQ.10 unit tests all pass; binary unit count = 147 + 10 = **157** |
| R.11 | Stage 12/13/14 invariants intact: 17 net-margin assessments still review, 7 manifests still manifest_ready, pending_operator_intents from Stage 14 demo unchanged |
| R.12 | Quality gates: `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p daemon --bin daemon` (157 binary tests), per-file integration loop green across 39 files |

---

## §6 — VALIDATION QUERIES

```bash
# Reset the 4 landed_cost_estimates rows to force re-computation with the new logic
sqlite3 state/state.db "
DELETE FROM process_stage_heads WHERE stage = 'landed cost';
DELETE FROM process_stage_status WHERE stage = 'landed cost';
DELETE FROM landed_cost_estimates;
"
cargo run -p daemon -- --ingest landed-cost
# Then trigger downstream rebuilds so Stage 12/13 see the new Stage 11 outputs:
cargo run -p daemon -- --ingest net-gate
cargo run -p daemon -- --ingest manifest

sqlite3 -header -column state/state.db "
SELECT '=== R.6/R.8/R.9 per-row basis + margin + confidence ===';
SELECT pc.name AS candidate,
       json_extract(lce.confidence_breakdown, '\$.order_quantity_basis.kind') AS basis_kind,
       lce.commercial_estimate_confidence_bps AS comm_conf,
       cpa.gross_margin_bps AS gross_bps,
       lce.net_margin_low_bps AS m_low, lce.net_margin_base_bps AS m_base, lce.net_margin_high_bps AS m_high
FROM landed_cost_estimates lce
JOIN candidate_pricing_assessments cpa ON cpa.product_candidate_id=lce.product_candidate_id AND cpa.selected_quote_id=lce.supplier_quote_id
JOIN product_candidates pc ON pc.id=lce.product_candidate_id
ORDER BY pc.name;

SELECT '=== R.5 BasisUndetermined warning emitted ===';
SELECT pc.name, lce.warnings AS warnings_json
FROM landed_cost_estimates lce
JOIN product_candidates pc ON pc.id=lce.product_candidate_id
WHERE lce.warnings LIKE '%QUANTITY_BASIS_UNDETERMINED_NO_OPERATOR_OVERRIDE%'
ORDER BY pc.name;

SELECT '=== R.7/R.11 Stage 12 verdicts intact ===';
SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY verdict;

SELECT '=== R.11 Stage 14 invariants ===';
SELECT idempotency_key, status FROM pending_operator_intents WHERE idempotency_key LIKE 'dec-2026-05-01-dgu-%';
SELECT product_candidate_id, payload_json FROM operator_input_overrides WHERE withdrawn_at IS NULL AND superseded_by_override_id IS NULL;
"
```

---

## §7 — FAILURE-MODE PLAYBOOK

| Symptom | Cause | Fix |
|---|---|---|
| `cargo check` errors on missing `match` arms after adding `BasisUndetermined` | Some `match OrderQuantityBasis` doesn't have a wildcard; new variant unhandled | Walk through every error; add the arm. The compiler enumerates them — let it lead the work |
| Aggregates still shows `gross_margin_bps=9999` | Persistence calibration not applied OR not in the right code path | The calibration must run wherever `gross_margin_bps` is computed in Stage 11 — likely in `build_landed_cost_estimate` or its `compute_*_landed_cost` callees. Trace from `gross_margin_bps =` assignment back to where basis is checked |
| DGU row regresses to `basis_undetermined` instead of `from_operator_override` | New inference logic ran before the Stage 14 override short-circuit | Re-read `infer_order_quantity_basis`'s Stage 14 short-circuit at the top of the function. The override check MUST happen before any other branch |
| Stage 12 verdicts changed for non-Aggregates rows | Confidence calibration too aggressive — applied to rows that shouldn't be affected | The calibration only fires for `BasisUndetermined` and `RfqRequired-without-override`. Other variants (FromContractValue, FromLotQuantity, FromOperatorOverride, SupplierCapacity) keep their existing confidence math |
| Modified `execute_landed_cost_skill_session` or `searates-freight-estimate/SKILL.md` | Out of scope — that's the captcha resilience handoff | Stop. Revert. This handoff is `infer_order_quantity_basis` + persistence calibration only |
| Schema epoch mismatch errors | Spurious migration added | This handoff adds NO migrations. If `cargo test` complains about epoch, you've changed something you shouldn't have — revert |

---

## §8 — CONSTRAINTS

1. **No new migrations.** This handoff is logic-only.
2. **No changes to `execute_landed_cost_skill_session`** or `skills/landed-cost/searates-freight-estimate/SKILL.md` — that's the captcha resilience handoff (running in parallel).
3. **No changes to Stage 14 code paths** — `pending_operator_intents`, `operator_input_overrides`, the CLI subcommands, IPC commands. The Stage 14 override short-circuit MUST be preserved at the top of `infer_order_quantity_basis`.
4. **Stage 12 verdict contract unchanged** — Aggregates must remain REVIEW; carpets must remain REVIEW; DGU must remain REVIEW. The reasons may shift slightly (the existing `quantity_basis_undetermined` check may or may not fire in addition to the new `BasisUndetermined` variant) but the verdict cannot regress.
5. **Use a worktree** if running concurrently with the captcha handoff: `git worktree add ../testproject1-stage11-orderqty main`. Merge to main with rebase at the end.
6. **No silent test exclusion.** Per AGENTS.md `### Stage Validation Test Gate`, the per-file integration loop is mandatory.

---

## §9 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/stage-11-hardening-order-quantity-validation-manifest.md`. Same template shape as the Stage 11 hardening manifest (docs/pipeline/stage-11-real-data-validation-manifest.md is the reference). Include before/after diff of the 4 landed_cost_estimates rows' basis_kind and gross_margin_bps.

### Quality gates

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon          # 157 tests
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
```

---

## §10 — REMINDERS

- ULTRATHINK and be meticulous.
- The goal is **honest numbers**, not changing Stage 12's verdict. Aggregates must stay REVIEW; what changes is the underlying gross_margin_bps (9999 → 0) and confidence (high → 1000).
- The Stage 14 operator override path is the proper long-term fix for Aggregates' quantity problem. This handoff is what Stage 11 does in the absence of an override.
- If you find yourself touching Stage 11's skill invocation, Stage 14's CLI, or any UI code — stop. Wrong handoff.
- Worktree for parallel safety. Rebase to main at the end.
