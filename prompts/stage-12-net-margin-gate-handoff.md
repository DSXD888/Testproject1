# Stage 12 — Net Margin Gate (Build, Validate, Test, Fix in Loop)

ULTRATHINK and be meticulous.

## MISSION

Build Stage 12 (Net Margin Gate) end-to-end against the existing live DB. Today the pipeline produces 4 `landed_cost_estimates` rows (3 domestic, 1 import — all `decision_grade=review`) and 13 `product_candidates` that were searched but produced no quote. Stage 12 turns that input set into per-candidate margin verdicts and a per-process coverage rollup that Stage 13's manifest can read.

You are not just building it. You will **build → validate → test → fix in a loop** until every numbered acceptance criterion (R.1–R.18 below) is GREEN. Do not stop with any criterion failing. Do not narrow the test gate to make it green.

```
┌──────────────────────────────────────────────────────────────────┐
│                          THE LOOP                                  │
│                                                                    │
│   1. Pre-flight (verify the facts in §0 below)                    │
│   2. Implement: migration + register + Rust + dispatch + tests    │
│   3. Run trigger: cargo run -p daemon -- --ingest net-gate        │
│   4. Validate against R.1..R.18 (queries inline below)            │
│   5. Any criterion failing?                                       │
│      ├─ YES → diagnose root cause, fix, clear stage tracking      │
│      │        if needed, go back to step 3                        │
│      └─ NO  → run quality gates (fmt + check + clippy + binary    │
│                unit tests + per-file integration loop). Failures? │
│                ├─ YES → fix, go to step 4                         │
│                └─ NO  → write the validation manifest             │
│                                                                    │
│   DO NOT STOP UNTIL ALL R.* PASS AND ALL QUALITY GATES ARE GREEN. │
└──────────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first. Pay particular attention to `## Compiler Checks (CRITICAL)` and `### Stage Validation Test Gate` — the binary-unit-test-only command is **insufficient**; the per-file integration loop is mandatory.

---

## §0 — PRE-FLIGHT (verify before you start)

These facts were confirmed against the live tree at handoff authoring time. Reconfirm before you write a line of code; if any has drifted, adjust accordingly and note it in the manifest.

| Claim | Verify command |
|---|---|
| Migration directory is `migrations/` at **repo root** (not `crates/daemon/migrations/`) | `ls migrations/` should show `0001_sqlite_state_initial.sql` ... `0006_pdf_extraction_toolkit.sql` |
| Highest existing migration is `0006_pdf_extraction_toolkit` (epoch 3) | `grep "MIGRATIONS:" -A 25 crates/daemon/src/db/sqlite.rs` |
| `landed_cost_estimates` is created in `0001`, not in a separate Stage 11 migration | `grep "CREATE TABLE IF NOT EXISTS landed_cost_estimates" migrations/0001_sqlite_state_initial.sql` |
| `pipeline_assessments` schema has the columns this handoff assumes (`pipeline_state`, `review_required`, `priced_coverage_bps`, etc.) | `sqlite3 state/state.db ".schema pipeline_assessments"` |
| `pipeline_assessments` is currently empty (Stage 12 is the first writer for net-gate purposes) | `sqlite3 state/state.db "SELECT COUNT(*) FROM pipeline_assessments"` |
| `allocate_stage_run_id()` lives in `crates/daemon/src/jobs/runner.rs:728` | `grep -n "fn allocate_stage_run_id" crates/daemon/src/jobs/runner.rs` |
| Stage 11 deterministic ID format: `format!("landed-{candidate_id}")` for upserts | `grep 'format!("landed-' crates/daemon/src/main.rs` |
| 4 priced candidates + 13 searched-unpriced + 3 service-excluded today | inline query in §3 below |
| `notice_lots.estimated_value_pence` is the lot value field for value-coverage | `sqlite3 state/state.db ".schema notice_lots"` |
| `pending_operator_intents` already exists (Stage 14 won't need to create it) — informational only | `sqlite3 state/state.db ".schema pending_operator_intents"` |
| **`pdf_t_schema_activation_009_default_schema_epoch_is_3` exists and asserts `DEFAULT_SCHEMA_EPOCH == 3`. This test WILL FAIL when you bump epoch to 4 in §2.2 — you must update or replace it** (see §2.2 step 4) | `grep -n "pdf_t_schema_activation_009" crates/daemon/tests/schema_activation.rs` |
| `persist_stage(...)` helper exists in `crates/daemon/src/main.rs:14196` — use it for stage tracking writes instead of inline SQL | `grep -n "fn persist_stage(" crates/daemon/src/main.rs` |
| `sha2 = "0.10.9"` is in `crates/daemon/Cargo.toml` — sha256 hashing is already available, no new dep needed | `grep "^sha2" crates/daemon/Cargo.toml` |

If any of those drift, the rest of this handoff still holds — just update the matching numbers in the validation queries.

---

## §1 — DECISIONS ALREADY MADE (DO NOT RE-OPEN)

The operator has signed off on D1–D6. Implement to spec; do not deviate.

### D1 — Verdict thresholds

| Verdict | Rule |
|---|---|
| **PASS** | `net_margin_base_bps >= 2000` AND `net_margin_low_bps >= 0` AND **all D2 confidence floors clear** AND **no D5 quantity-basis blocker** AND no awaiting-input reason |
| **REJECT** | `net_margin_high_bps < 500` (best case can't clear the review floor) AND no awaiting-input reason AND no quantity-basis blocker |
| **REVIEW** | everything else (covers REVIEW band, plus all "needs operator input" cases) |

Threshold numbers are: `PASS_THRESHOLD_BPS = 2000`, `REJECT_THRESHOLD_BPS = 500`. Snapshot both into every persisted row (`pass_threshold_bps`, `reject_threshold_bps`) so a re-run with new numbers is detectable via basis_hash.

### D2 — Confidence model (lex-gated, hard floors)

Each axis is checked independently. Failing **any one** demotes the candidate to REVIEW with a structured reason `confidence_<axis>_below_floor`. PASS is impossible unless all four floors clear.

| Axis | Source | Floor (bps) | Reason code |
|---|---|---|---|
| `revenue_confidence_bps` | `candidate_pricing_assessments` | 5000 | `confidence_revenue_below_floor` |
| `pricing_input_confidence_bps` | `candidate_pricing_assessments` | 5000 | `confidence_pricing_input_below_floor` |
| `customs_resolution_confidence_bps` | `landed_cost_estimates` | 6000 | `confidence_customs_resolution_below_floor` |
| `commercial_estimate_confidence_bps` | `landed_cost_estimates` | 4000 | `confidence_commercial_estimate_below_floor` |

"Below floor" means **strictly less than** (`<`), not `<=`. So a value exactly at the floor (e.g. Aggregates' `commercial_estimate_confidence_bps = 4000` against floor 4000) **passes**.

If a confidence axis is `NULL`, treat as below-floor — **except for `no_pricing_signal` candidates (D4), where confidence-floor evaluation is skipped entirely** (they have no pricing inputs by definition; the only reason emitted is `no_pricing_signal`).

### D3 — RFQ-required policy

When `candidate_pricing_assessments.revenue_basis_type = 'rfq_required'` **or** `landed_cost_estimates.confidence_breakdown.order_quantity_basis.kind = 'rfq_required'`:

- Verdict: **REVIEW**
- Reason code: `awaiting_rfq_quote`
- Margin band recorded as observed (likely 0/0/0) but not used to derive the verdict
- `re_evaluation_eligible = 1` so Stage 14 knows this row can be re-run with operator input

### D4 — No-pricing-signal candidates

Stage 12 produces a row for every candidate that **at least entered the pricing pipeline**. Concretely the input set is the union of:

1. Candidates with a `candidate_pricing_assessments` row (priced — 4 today).
2. Candidates with at least one `search_attempts` row but no `candidate_pricing_assessments` row (searched, no quote — 13 today).

For (2): verdict = **REVIEW**, reasons = `["no_pricing_signal"]` (single-reason, no confidence floors evaluated). Margin band fields, revenue, economic landed cost, and all four confidence axes are all `NULL`. The point is **operator visibility**: the operator must be able to see "we tried to price this; we found nothing" without having to drill into `search_attempts`. `re_evaluation_eligible = 1`.

Candidates with a `search_profiles` row but no `search_attempts` (~800 today) are **not** processed by Stage 12. They surface at the lot/process level via the coverage rollup metrics (D6) — Stage 13's manifest will render them as "not yet searched". This avoids inflating the review queue with 800 "we haven't tried yet" rows.

Service candidates (`product_candidates.candidate_type = 'service'`) are excluded — they were already filtered by Stage 9.

### D5 — Quantity-basis sanity check

If revenue and supplier-cost are not on the same quantity basis, the computed margin is mathematically real but operationally meaningless (e.g. Aggregates today: revenue £167k = framework total over multiple years, supplier_cost £15.40 = single pack from amazon-uk). Detect and force REVIEW.

Apply in this order; first match wins for the **reason code**, but the verdict is always REVIEW:

| Condition | Reason code |
|---|---|
| `landed_cost_estimates.confidence_breakdown.order_quantity_basis.kind` IN (`'rfq_required'`, `'supplier_capacity'`) | `quantity_basis_unaligned` |
| `landed_cost_estimates.confidence_breakdown.order_quantity_basis` is NULL AND `candidate_pricing_assessments.revenue_basis_type = 'expected_value'` AND `supplier_quotes.price_basis_qty` IS NOT NULL | `quantity_basis_undetermined` |
| `kind` IN (`'from_contract_value'`, `'from_lot_quantity'`) | (no reason; trust the band) |

Reasons stack — a candidate can be `awaiting_rfq_quote` AND `quantity_basis_unaligned` AND `confidence_revenue_below_floor` simultaneously. The reasons array preserves all of them in the order: `[quantity_basis_*, awaiting_rfq_quote, no_pricing_signal, confidence_*_below_floor (alphabetical), margin_below_*]`. This is the **deterministic precedence** used to pick `pipeline_assessments.review_reason` (most-blocking first wins).

### D6 — Coverage rollup (per-process, written to `pipeline_assessments`)

Folded into Stage 12 (per L0 — "coverage rollup runs inline as part of `compute_net_gate`"). For every process that has at least one Stage 12 candidate row, write/upsert `pipeline_assessments` (PRIMARY KEY = `process_id`):

**Coverage definitions** (per L0 §3):

| Field | Formula |
|---|---|
| `priced_coverage_bps` | (lots with ≥1 PASS-or-REVIEW priced candidate) / total qualifying lots × 10000 |
| `landed_cost_coverage_bps` | (lots with ≥1 candidate that has a landed_cost row) / total qualifying lots × 10000 |
| `net_qualified_coverage_bps` | (lots with ≥1 PASS candidate) / total qualifying lots × 10000 |
| `priced_value_coverage_bps` | (sum of `notice_lots.estimated_value_pence` for lots with PASS-or-REVIEW priced candidate) / (sum of `estimated_value_pence` for all qualifying lots) × 10000 |
| `landed_cost_value_coverage_bps` | analogous, lots with landed cost |
| `net_qualified_value_coverage_bps` | analogous, lots with PASS candidate |
| `unresolved_mandatory_lots` | count of qualifying lots with **0 PASS-or-REVIEW priced candidates** (`no_pricing_signal` REVIEWs do **not** count as resolving the lot) |
| `gross_margin_bps` | weighted avg over PASS+REVIEW priced candidates of `candidate_pricing_assessments.gross_margin_bps`, weighted by `product_candidates.candidate_value_share_bps`. Use simple avg if shares are NULL |
| `net_margin_bps` | analogous, using `candidate_net_margin_assessments.net_margin_base_bps` |
| `pipeline_state` | always `'net_qualified'` (per L0 enum) when Stage 12 has run |
| `review_required` | `1` if any candidate-row verdict is REVIEW; else `0` |
| `review_stage` | `'net_margin_gate'` when `review_required = 1`; else NULL |
| `review_reason` | the precedence-winning reason across child candidates (per D5 ordering) |
| `decision_basis_hash` | `sha256(sorted child basis_hashes \|\| pass_threshold \|\| reject_threshold \|\| thresholds_version \|\| rollup_version)` |

**"Qualifying lots"** = lots whose latest `process_stage_status` row for `(stage='qualify lot', scope_kind='lot')` has `state='succeeded'`. There is no `notice_lots.mandatory` flag in MVP; treat all qualifying lots as mandatory. If/when a flag lands, this rule changes.

**Reasons-array canonical form** (used in §3 expected results and verified by R.3): JSON string array, no whitespace, double-quoted strings, sorted by D5 precedence with confidence-floor reasons alphabetical within their bucket. Example: `["awaiting_rfq_quote","confidence_commercial_estimate_below_floor","confidence_pricing_input_below_floor","confidence_revenue_below_floor"]`. Use a deterministic JSON serializer (e.g. `serde_json::to_string` after building a `Vec<&'static str>`).

**Edge case — 0 qualifying lots in a process**: set all coverage_bps to 0 and `unresolved_mandatory_lots = 0`. Do not divide by zero.

**Edge case — qualifying lots all have `estimated_value_pence` NULL**: set value-coverage metrics to 0 (cannot weight by value); count-based coverage still applies.

**`pipeline_assessments.validation_state`** has a NOT NULL DEFAULT `'unverified'`. Stage 12's UPSERT should not overwrite this column; preserve whatever's there (or leave default on first insert). Stage 11 already wires this from a `live_validation` context — see the existing INSERT INTO pipeline_assessments at `crates/daemon/src/main.rs:11869` as the canonical INSERT pattern (Stage 11's manifest path uses `pipeline_state='manifest_ready'`; Stage 12 uses `'net_qualified'`).

**`pipeline_state` regression note**: Stage 12 unconditionally sets `pipeline_state='net_qualified'`. If Stage 13 has previously run and set `'manifest_ready'`, a Stage 12 re-run will downgrade it to `'net_qualified'`. This is acceptable in MVP because Stage 13 should re-run whenever Stage 12 changes its basis_hash. Stage 13's handoff will need to handle this re-run cascade.

---

## §2 — DESIGN AT A GLANCE

```
                    candidate_pricing_assessments  (Stage 10)
                              │
                              ▼
                    landed_cost_estimates          (Stage 11)
                              │
       ┌──────────────────────┼──────────────────────────┐
       │  Stage 12: per-candidate verdict                │
       │  → candidate_net_margin_assessments  (NEW — 0007 migration)
       └──────────────────────┬──────────────────────────┘
                              │
                              ▼
                     per-process rollup (Stage 12)
                              │
                              ▼
                  pipeline_assessments  (EXISTS, currently empty)
```

Two outputs per Stage 12 invocation: per-candidate rows and per-process rollup rows. Both written under one `stage_run_id` per invocation (set-replacement semantics — see §2.3).

### §2.1 — New migration: `migrations/0007_net_margin_gate_assessments.sql`

```sql
BEGIN;

CREATE TABLE IF NOT EXISTS candidate_net_margin_assessments (
  id TEXT PRIMARY KEY,                     -- deterministic: 'net-margin-{product_candidate_id}'
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  notice_id TEXT NOT NULL REFERENCES contract_notices(id),
  lot_id TEXT REFERENCES notice_lots(id),
  product_candidate_id TEXT NOT NULL REFERENCES product_candidates(id),
  pricing_assessment_id TEXT REFERENCES candidate_pricing_assessments(id),
  landed_cost_estimate_id TEXT REFERENCES landed_cost_estimates(id),

  verdict TEXT NOT NULL,                   -- 'pass' | 'review' | 'reject'
  reasons_json TEXT NOT NULL DEFAULT '[]', -- JSON string[] of reason codes (D2/D3/D4/D5)
  re_evaluation_eligible INTEGER NOT NULL DEFAULT 0,

  net_margin_low_bps INTEGER,
  net_margin_base_bps INTEGER,
  net_margin_high_bps INTEGER,

  revenue_used_pence INTEGER,
  revenue_basis_type TEXT,
  economic_landed_cost_used_pence INTEGER,

  revenue_confidence_bps INTEGER,
  pricing_input_confidence_bps INTEGER,
  customs_resolution_confidence_bps INTEGER,
  commercial_estimate_confidence_bps INTEGER,

  pass_threshold_bps INTEGER NOT NULL,
  reject_threshold_bps INTEGER NOT NULL,
  thresholds_version TEXT NOT NULL,        -- bump when D1/D2 change; e.g., 'v1.0-2026-05-01'
  basis_hash TEXT NOT NULL,

  stage_run_id TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_cnma_process ON candidate_net_margin_assessments(process_id);
CREATE INDEX IF NOT EXISTS idx_cnma_verdict ON candidate_net_margin_assessments(verdict);
CREATE INDEX IF NOT EXISTS idx_cnma_basis  ON candidate_net_margin_assessments(basis_hash);

COMMIT;
```

**No UNIQUE per-stage-run constraint** — set-replacement is via deterministic `id` + `INSERT ... ON CONFLICT(id) DO UPDATE`, mirroring Stage 11's pattern (see `crates/daemon/src/main.rs` for `format!("landed-{candidate_id}")`). Re-running Stage 12 for the same candidate replaces the row in place; the latest `stage_run_id` is recorded but no historical rows accumulate.

### §2.2 — Migration registration (REQUIRED, easy to forget)

In `crates/daemon/src/db/sqlite.rs`, follow the exact pattern used for `0006`:

1. Add a constant near the existing `L_PDF_MIGRATION_PATH`:
   ```rust
   const L_NET_GATE_MIGRATION_PATH: &str = "migrations/0007_net_margin_gate_assessments.sql";
   ```
2. Add an entry to the `MIGRATIONS` array (after `0006`):
   ```rust
   ("0007_net_margin_gate_assessments", 4, L_NET_GATE_MIGRATION_PATH),
   ```
   Schema epoch is `4` (each new-table migration bumps epoch in this codebase: `0005`=2, `0006`=3, so `0007`=4).
3. Update `pub const DEFAULT_SCHEMA_EPOCH: i64` to `4`.
4. **Update or replace `pdf_t_schema_activation_009_default_schema_epoch_is_3`** in `crates/daemon/tests/schema_activation.rs:1026`. That test hard-codes `DEFAULT_SCHEMA_EPOCH == 3` and `user_version == 3`; once you bump to 4 it will fail. Either rename the existing test to `_004_default_schema_epoch_is_4` and assert `== 4`, OR add a new test with the new assertion and update the old one to assert `>= 3`. Do not delete the old test without replacement — schema epoch is a constitutional invariant.

Without these four steps, the migration won't run and/or `cargo test` will go red on an unrelated-looking test.

### §2.3 — Re-runnability via `basis_hash`

```
basis_hash = sha256(
  product_candidate_id || ':' ||
  COALESCE(landed_cost_estimate_id, '<none>') || ':' ||
  COALESCE(landed_cost_estimates.row_version::text, '0') || ':' ||
  COALESCE(pricing_assessment_id, '<none>') || ':' ||
  COALESCE(candidate_pricing_assessments.row_version::text, '0') || ':' ||
  thresholds_version
)
```

Inputs are sorted/canonical (single deterministic order). If the freshly-computed `basis_hash` for a candidate matches what's already persisted, **skip** (idempotent no-op). Otherwise upsert.

### §2.4 — Stage tracking

Two scope kinds (consistent with L0 stage scope matrix):

- `process_stage_status` / `process_stage_heads`:
  - stage = `'net margin gate'`, scope_kind = `'candidate'`, scope_id = `product_candidates.candidate_key` (deterministic natural key — **NOT** `product_candidates.id`) — one per candidate processed
  - stage = `'net margin gate'`, scope_kind = `'process'`, scope_id = `procurement_processes.id` — one per process rolled up

State is `'succeeded'` for both on a normal run.

**Use the existing `persist_stage(tx, process_id, stage, scope_kind, scope_id, stage_run_id)` helper at `crates/daemon/src/main.rs:14196`** — it writes both `process_stage_status` and `process_stage_heads` correctly with the canonical `ON CONFLICT(process_id, stage, scope_kind, scope_id) DO UPDATE` pattern. Do not write inline SQL; reuse the helper.

`persist_stage` is **module-private** (no `pub`), so `run_net_margin_gate_live_loop` and any sub-functions you write must live in `crates/daemon/src/main.rs`. If you later want to extract Stage 12 into its own module, you'd need to also re-locate `persist_stage` — out of scope for this handoff.

**`process_stage_heads.basis_hash` is a real column** but the existing `persist_stage` helper does not write it (it takes only the 6 parameters listed above). For MVP, **store basis_hash only on `candidate_net_margin_assessments`** (already in the migration); the skip-on-match check reads from there. Extending `persist_stage` to accept and write basis_hash is out of scope for this handoff — that's a cross-cutting change that other stages would need too.

**`lot_id` FK vs `lot` `scope_id` distinction**: the `candidate_net_margin_assessments.lot_id` column is a foreign key to `notice_lots.id` (the surrogate PK). The `process_stage_heads.scope_id` for `scope_kind='lot'` (used by Stage 7) is `notice_lots.lot_key` (the natural key). They are different fields; do not confuse them.

### §2.5 — Dispatch

Add `--ingest net-gate` (mirrors `--ingest landed-cost` at `crates/daemon/src/main.rs:1161`).

```rust
// in the existing match block around line 1159:
"net-gate" => run_net_margin_gate_live_loop(&mut runtime, &cli).await?,
```

Implement:

```rust
async fn run_net_margin_gate_live_loop(
    runtime: &mut RuntimeBootstrap,
    cli: &Cli,
) -> Result<LiveLoopReport> {
    let stage_run_id = allocate_stage_run_id();  // import from crate::jobs::runner
    // 1. Load input set (priced + searched-but-unpriced, ex-service)
    // 2. For each candidate:
    //    a. compute basis_hash; skip if matches persisted
    //    b. compute verdict per D1..D5; build reasons array (deterministic order per D5)
    //    c. UPSERT candidate_net_margin_assessments with id = format!("net-margin-{candidate_id}")
    //    d. write process_stage_status / process_stage_heads at scope_kind='candidate'
    // 3. For each affected process:
    //    a. compute coverage rollup per D6
    //    b. UPSERT pipeline_assessments with ON CONFLICT(process_id) DO UPDATE
    //    c. write process_stage_status / process_stage_heads at scope_kind='process'
    // 4. Return summary line
}
```

Summary line shape (matches Stage 11):

```
net-margin-gate live loop completed: attempted=N candidates_assessed=M
  pass=P review=R reject=J no_pricing_signal=K
  processes_rolled_up=X skipped_unchanged=S
```

**Wiring `--ingest net-gate` into `run_deterministic_pipeline` is OUT OF SCOPE for this handoff.** Standalone trigger only. Pipeline integration is a follow-up bead.

---

## §3 — EXPECTED RESULTS AGAINST CURRENT DATA

After a clean run today, the DB should look like this. Use these as your reference for verifying R.1–R.18:

| Candidate | Inputs | Verdict | Reasons |
|---|---|---|---|
| Aggregates (general) | priced, domestic, revenue=£167k, gross=9999 bps, no `order_quantity_basis` recorded by Stage 11. Confidences: rev=6500 ✓, price=5250 ✓, customs=10000 ✓, comm=4000 ✓ (= floor; passes per strict-less-than rule). `supplier_quotes.price_basis_qty=17.5 kg`, `revenue_basis_type='expected_value'` → D5 row 2 fires | **REVIEW** | `["quantity_basis_undetermined"]` |
| Double-glazed unit | priced, import, `revenue_basis_type='rfq_required'`, `order_quantity_basis.kind='rfq_required'`. Confidences: rev=1000 ✗, price=1000 ✗, customs=8400 ✓, comm=6500 ✓. D5 row 1 fires; D3 fires; both confidence floors fail | **REVIEW** | `["quantity_basis_unaligned", "awaiting_rfq_quote", "confidence_pricing_input_below_floor", "confidence_revenue_below_floor"]` (per D5 ordering: quantity_basis → awaiting → confidence alphabetical) |
| Ely Library carpet flooring package | priced, domestic, `revenue_basis_type='rfq_required'`. Confidences: rev=1000 ✗, price=1000 ✗, customs=10000 ✓, comm=1000 ✗. No `order_quantity_basis` (domestic), but D5 row 2 needs `revenue_basis_type='expected_value'` which is false → D5 doesn't fire. D3 fires; three confidence floors fail | **REVIEW** | `["awaiting_rfq_quote", "confidence_commercial_estimate_below_floor", "confidence_pricing_input_below_floor", "confidence_revenue_below_floor"]` |
| Library carpet floor covering | priced, domestic, `revenue_basis_type='rfq_required'`. Confidences identical to Ely Library (rev=1000, price=1000, customs=10000, comm=1000) | **REVIEW** | `["awaiting_rfq_quote", "confidence_commercial_estimate_below_floor", "confidence_pricing_input_below_floor", "confidence_revenue_below_floor"]` |
| 13 unpriced-but-searched candidates | searched, no quote | **REVIEW** | `["no_pricing_signal"]` (single-element; no confidence floors evaluated per D2) |

**Total expected `candidate_net_margin_assessments` rows: 17.**
**Total expected PASS: 0** (Aggregates would have been PASS but D5 forces REVIEW).
**Total expected REJECT: 0** today (no candidate has a band that REJECT-clears the threshold; everything is upstream-gated).
**`pipeline_assessments` rollup**: one row per distinct process across the 17 candidates. `pipeline_state='net_qualified'`, `review_required=1`, `review_stage='net_margin_gate'`.

This **0 PASS / 17 REVIEW / 0 REJECT** outcome is **correct**, not a bug. It accurately reflects that today's data needs operator input (quantity overrides, RFQ quotes, manual pricing) before any candidate can auto-pass.

If your implementation produces a PASS today, **D5 is misfiring** — re-read it.

---

## §4 — REQUIRED NEW UNIT TESTS

Stage 11 hardening shipped 5 new code paths without unit tests; that became a real defect. Stage 12 must not repeat that. Add these to `crates/daemon/src/main.rs` (`mod tests`) — every test name listed must exist, with a `#[test]` or `#[tokio::test]` attribute, and pass.

| # | Test name | What it verifies |
|---|---|---|
| T.1 | `net_margin_pass_when_base_meets_threshold_and_low_nonneg_and_floors_clear` | Happy path PASS: synthetic row with base=2500, low=100, high=4000, all 4 confidences ≥ floor → PASS, reasons=[] |
| T.2 | `net_margin_review_when_base_below_pass` | base=1500, low=100 → REVIEW with `margin_below_pass` |
| T.3 | `net_margin_review_when_low_negative` | base=2500, low=-200 → REVIEW with `margin_below_pass` (the band-includes-loss case demotes to REVIEW even though base passes) |
| T.4 | `net_margin_reject_when_high_below_reject_threshold` | high=400 → REJECT |
| T.5 | `net_margin_review_when_revenue_confidence_below_floor` | rev_conf=4000, all else PASS-shape → REVIEW with `confidence_revenue_below_floor` |
| T.6 | `net_margin_review_when_customs_confidence_below_floor` | customs_conf=5500 → REVIEW with `confidence_customs_resolution_below_floor` |
| T.7 | `net_margin_review_when_rfq_required_revenue_basis` | revenue_basis_type='rfq_required' → REVIEW with `awaiting_rfq_quote`; margin verdict skipped |
| T.8 | `net_margin_review_when_quantity_basis_unaligned` | order_quantity_basis.kind='rfq_required' → REVIEW with `quantity_basis_unaligned` |
| T.9 | `net_margin_review_when_quantity_basis_undetermined` | order_quantity_basis is NULL AND revenue_basis_type='expected_value' AND price_basis_qty is set → REVIEW with `quantity_basis_undetermined` |
| T.10 | `net_margin_no_pricing_signal_skips_confidence_floors` | candidate has search_attempt but no pricing_assessment → REVIEW with reasons=["no_pricing_signal"] **only** (no `confidence_*_below_floor`) |
| T.11 | `basis_hash_stable_across_runs_with_identical_inputs` | Build a synthetic input twice; assert hashes equal |
| T.12 | `basis_hash_changes_when_landed_cost_row_version_changes` | Bump `landed_cost_estimates.row_version`; assert hash differs |
| T.13 | `idempotent_no_op_when_basis_hash_matches_persisted` | Run Stage 12 twice on same fixture; second call upserts 0 rows |
| T.14 | `coverage_rollup_zero_qualifying_lots_yields_zero_metrics_no_panic` | Process with 0 qualifying lots; coverage_bps fields all 0; no divide-by-zero |
| T.15 | `coverage_rollup_priced_value_weights_by_lot_estimated_value` | 2 lots: A value=£1000 has PASS candidate, B value=£3000 has none. priced_value_coverage_bps = 2500 |
| T.16 | `coverage_rollup_unresolved_mandatory_excludes_no_pricing_signal_reviews` | Lot has only no_pricing_signal candidates → counted as unresolved (not as resolved-by-REVIEW) |
| T.17 | `review_reason_precedence_picks_quantity_basis_over_confidence` | Candidate has `quantity_basis_unaligned` AND `confidence_revenue_below_floor`; pipeline_assessments.review_reason = `quantity_basis_unaligned` |
| T.18 | `reasons_array_serializes_to_json_string_array_in_deterministic_order` | reasons = `["awaiting_rfq_quote", "confidence_revenue_below_floor"]` always serializes the same way (per D5 ordering) |

**These tests must run under `cargo test -p daemon --bin daemon` with no special setup.** Tests requiring full DB fixtures should follow the same `live_validation_test_skill`-style helpers Stage 11 tests use.

---

## §5 — REQUIRED SCHEMA-ACTIVATION TEST

Add a test to `crates/daemon/tests/schema_activation.rs` mirroring `pdf_t_schema_activation_002_upgrade_path_applies_0006_cleanly` (see that test for the exact pattern):

```rust
#[test]
fn ngg_t_schema_activation_007_upgrade_path_applies_0007_cleanly() -> Result<()> {
    // Apply 0001..0006 manually; seed schema_migrations to epoch 3;
    // open StateDb (which should apply 0007 + bump epoch to 4);
    // assert candidate_net_margin_assessments table exists with the expected columns;
    // assert schema_migrations row for '0007_net_margin_gate_assessments' exists.
}
```

---

## §6 — ACCEPTANCE CRITERIA — R.1 to R.18

Each must PASS before declaring done. Validation queries inline in §7.

| # | Criterion |
|---|---|
| R.1 | `candidate_net_margin_assessments` row count = 4 + N where `N = COUNT(DISTINCT pc.id) FROM product_candidates pc JOIN search_attempts sa ON sa.product_candidate_id = pc.id LEFT JOIN candidate_pricing_assessments cpa ON cpa.product_candidate_id = pc.id WHERE cpa.id IS NULL AND pc.candidate_type != 'service'` (today: 4+13=17) |
| R.2 | Verdict distribution matches expected (today: 0 pass, 17 review, 0 reject) |
| R.3 | Every reason emitted is from the closed enum: `confidence_{revenue,pricing_input,customs_resolution,commercial_estimate}_below_floor`, `awaiting_rfq_quote`, `quantity_basis_unaligned`, `quantity_basis_undetermined`, `no_pricing_signal`, `margin_below_pass`. No other strings appear |
| R.4 | Aggregates row has `quantity_basis_undetermined`; verdict = REVIEW; `re_evaluation_eligible = 1` |
| R.5 | All 3 RFQ-required priced rows have `awaiting_rfq_quote`; verdict = REVIEW; `re_evaluation_eligible = 1` |
| R.6 | All 13 unpriced-searched rows have `reasons_json = '["no_pricing_signal"]'` (exact); margin band fields are NULL; confidence axes NULL; `re_evaluation_eligible = 1` |
| R.7 | For every priced row, confidence-floor reasons appear iff the corresponding axis is below floor (cross-check programmatically) |
| R.8 | `pipeline_assessments` row count = number of distinct processes across the 17 candidates; FK integrity to `procurement_processes` |
| R.9 | All `pipeline_assessments` rows have `pipeline_state='net_qualified'`, `review_required=1`, `review_stage='net_margin_gate'`, non-NULL `review_reason` from the closed enum |
| R.10 | Coverage metrics computed correctly per D6 — verify: priced_coverage_bps, landed_cost_coverage_bps, net_qualified_coverage_bps, priced_value_coverage_bps, unresolved_mandatory_lots all non-NULL and within `[0, 10000]` (or 0 for value-coverage if estimated_value_pence missing) |
| R.11 | `process_stage_status` rows: count at scope_kind='candidate' = R.1 count; count at scope_kind='process' = R.8 count; all `state='succeeded'` |
| R.12 | `process_stage_heads` rows: same counts as R.11 (one head per scope) |
| R.13 | FK integrity: 0 orphans `candidate_net_margin_assessments` → process / notice / candidate / (pricing_assessment if set) / (landed_cost_estimate if set) |
| R.14 | Re-runnability: deleting all Stage 12 rows + re-running `--ingest net-gate` reproduces the same row count, same per-candidate verdicts, same reasons (deterministic) |
| R.15 | Idempotency: a second `--ingest net-gate` invocation produces zero new candidate rows AND zero pipeline_assessments mutations (basis_hash matches → skip). Log line reports `skipped_unchanged=R.1_count` |
| R.16 | Stage 11 untouched: 4 `landed_cost_estimates` still `decision_grade='review'`, all hardening fields unchanged. Specifically the DGU row (`supplier_quote_id='sq-b22fcf00b7a545d699c3438c42b9027c-1'`) must still show `supplier_cost=5160, freight_base=17692, customs_duty=0, import_vat=4570, economic_landed_cost=27422` |
| R.17 | Quality gates pass: `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p daemon --bin daemon` (T.1–T.18 included → 81+18=99 binary unit tests pass) |
| R.18 | Per-file integration loop is GREEN: `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" \|\| exit 1; done` returns exit 0 across all 39 files (must include the new schema_activation test from §5) |

---

## §7 — VALIDATION QUERIES

Run these after each `--ingest net-gate` attempt:

```bash
sqlite3 -header -column state/state.db "
SELECT '=== R.1 row count ===';
SELECT COUNT(*) AS cnma_rows FROM candidate_net_margin_assessments;
SELECT 'expected: ' || (4 + (
    SELECT COUNT(DISTINCT pc.id) FROM product_candidates pc
    JOIN search_attempts sa ON sa.product_candidate_id = pc.id
    LEFT JOIN candidate_pricing_assessments cpa ON cpa.product_candidate_id = pc.id
    WHERE cpa.id IS NULL AND pc.candidate_type != 'service'
));

SELECT '=== R.2 verdict distribution ===';
SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY verdict;

SELECT '=== R.3 reason audit (every distinct token must be in the closed enum) ===';
SELECT je.value AS token, COUNT(*) AS occurrences
FROM candidate_net_margin_assessments cnma
CROSS JOIN json_each(cnma.reasons_json) AS je
GROUP BY je.value
ORDER BY je.value;

SELECT '=== R.4 Aggregates ===';
SELECT cnma.verdict, cnma.reasons_json, cnma.re_evaluation_eligible
FROM candidate_net_margin_assessments cnma
JOIN product_candidates pc ON pc.id = cnma.product_candidate_id
WHERE pc.name LIKE 'Aggregates%';

SELECT '=== R.5 RFQ rows ===';
SELECT substr(pc.name, 1, 40), cnma.verdict, cnma.reasons_json
FROM candidate_net_margin_assessments cnma
JOIN product_candidates pc ON pc.id = cnma.product_candidate_id
JOIN candidate_pricing_assessments cpa
  ON cpa.id = cnma.pricing_assessment_id
WHERE cpa.revenue_basis_type = 'rfq_required'
ORDER BY pc.name;

SELECT '=== R.6 no_pricing_signal sample (5) ===';
SELECT substr(pc.name, 1, 40) AS name, cnma.verdict, cnma.reasons_json,
       cnma.net_margin_base_bps AS should_be_null,
       cnma.revenue_confidence_bps AS conf_should_be_null
FROM candidate_net_margin_assessments cnma
JOIN product_candidates pc ON pc.id = cnma.product_candidate_id
WHERE cnma.pricing_assessment_id IS NULL
ORDER BY pc.name LIMIT 5;

SELECT '=== R.7 confidence floor cross-check (bidirectional) ===';
-- Bidirectional: missing reason when value below floor (false negative),
-- AND spurious reason when value above floor (false positive).
SELECT
  -- Missing reasons (value below floor but reason absent):
  SUM(CASE WHEN cpa.revenue_confidence_bps < 5000 AND
                  reasons_json NOT LIKE '%confidence_revenue_below_floor%'
            THEN 1 ELSE 0 END) AS missing_revenue_floor,
  SUM(CASE WHEN cpa.pricing_input_confidence_bps < 5000 AND
                  reasons_json NOT LIKE '%confidence_pricing_input_below_floor%'
            THEN 1 ELSE 0 END) AS missing_pricing_floor,
  SUM(CASE WHEN lce.customs_resolution_confidence_bps < 6000 AND
                  reasons_json NOT LIKE '%confidence_customs_resolution_below_floor%'
            THEN 1 ELSE 0 END) AS missing_customs_floor,
  SUM(CASE WHEN lce.commercial_estimate_confidence_bps < 4000 AND
                  reasons_json NOT LIKE '%confidence_commercial_estimate_below_floor%'
            THEN 1 ELSE 0 END) AS missing_commercial_floor,
  -- Spurious reasons (value at-or-above floor but reason present):
  SUM(CASE WHEN cpa.revenue_confidence_bps >= 5000 AND
                  reasons_json LIKE '%confidence_revenue_below_floor%'
            THEN 1 ELSE 0 END) AS spurious_revenue_floor,
  SUM(CASE WHEN cpa.pricing_input_confidence_bps >= 5000 AND
                  reasons_json LIKE '%confidence_pricing_input_below_floor%'
            THEN 1 ELSE 0 END) AS spurious_pricing_floor,
  SUM(CASE WHEN lce.customs_resolution_confidence_bps >= 6000 AND
                  reasons_json LIKE '%confidence_customs_resolution_below_floor%'
            THEN 1 ELSE 0 END) AS spurious_customs_floor,
  SUM(CASE WHEN lce.commercial_estimate_confidence_bps >= 4000 AND
                  reasons_json LIKE '%confidence_commercial_estimate_below_floor%'
            THEN 1 ELSE 0 END) AS spurious_commercial_floor
FROM candidate_net_margin_assessments cnma
JOIN candidate_pricing_assessments cpa ON cpa.id = cnma.pricing_assessment_id
LEFT JOIN landed_cost_estimates lce ON lce.id = cnma.landed_cost_estimate_id;
-- All eight columns should be 0.

SELECT '=== R.8/R.9 pipeline_assessments rollup ===';
SELECT process_id, pipeline_state, review_required, review_stage, review_reason,
       priced_coverage_bps, net_qualified_coverage_bps, unresolved_mandatory_lots
FROM pipeline_assessments
ORDER BY process_id LIMIT 10;
SELECT 'pipeline_assessments count: ' || COUNT(*) FROM pipeline_assessments;
SELECT 'expected: ' || (
  SELECT COUNT(DISTINCT pc.process_id) FROM candidate_net_margin_assessments c
  JOIN product_candidates pc ON pc.id = c.product_candidate_id
);

SELECT '=== R.10 coverage bounds ===';
SELECT 'rows out of bounds: ' || COUNT(*) FROM pipeline_assessments
WHERE priced_coverage_bps < 0 OR priced_coverage_bps > 10000
   OR net_qualified_coverage_bps < 0 OR net_qualified_coverage_bps > 10000
   OR landed_cost_coverage_bps < 0 OR landed_cost_coverage_bps > 10000;

SELECT '=== R.11/R.12 stage tracking ===';
SELECT stage, scope_kind, state, COUNT(*) FROM process_stage_status
WHERE stage = 'net margin gate' GROUP BY stage, scope_kind, state;
SELECT stage, scope_kind, COUNT(*) FROM process_stage_heads
WHERE stage = 'net margin gate' GROUP BY stage, scope_kind;

SELECT '=== R.13 FK orphans ===';
SELECT 'orphan process: '   || COUNT(*) FROM candidate_net_margin_assessments c LEFT JOIN procurement_processes p ON p.id=c.process_id WHERE p.id IS NULL;
SELECT 'orphan notice: '    || COUNT(*) FROM candidate_net_margin_assessments c LEFT JOIN contract_notices n ON n.id=c.notice_id WHERE n.id IS NULL;
SELECT 'orphan candidate: ' || COUNT(*) FROM candidate_net_margin_assessments c LEFT JOIN product_candidates pc ON pc.id=c.product_candidate_id WHERE pc.id IS NULL;
SELECT 'orphan pricing: '   || COUNT(*) FROM candidate_net_margin_assessments c LEFT JOIN candidate_pricing_assessments cpa ON cpa.id=c.pricing_assessment_id WHERE c.pricing_assessment_id IS NOT NULL AND cpa.id IS NULL;
SELECT 'orphan landed: '    || COUNT(*) FROM candidate_net_margin_assessments c LEFT JOIN landed_cost_estimates lce ON lce.id=c.landed_cost_estimate_id WHERE c.landed_cost_estimate_id IS NOT NULL AND lce.id IS NULL;

SELECT '=== R.16 Stage 11 untouched ===';
SELECT decision_grade, supplier_cost_gbp_pence, freight_base_gbp_pence,
       customs_duty_gbp_pence, import_vat_gbp_pence, economic_landed_cost_gbp_pence
FROM landed_cost_estimates
WHERE supplier_quote_id = 'sq-b22fcf00b7a545d699c3438c42b9027c-1';
-- Expected: review|5160|17692|0|4570|27422 (per Stage 11 hardening manifest)
"
```

R.14 (re-runnability — destructive, only after R.1–R.13 are green):

```bash
sqlite3 state/state.db "
DELETE FROM candidate_net_margin_assessments;
DELETE FROM pipeline_assessments;
DELETE FROM process_stage_heads WHERE stage = 'net margin gate';
DELETE FROM process_stage_status WHERE stage = 'net margin gate';
"
cargo run -p daemon -- --ingest net-gate
# Re-run R.1–R.6 and confirm identical counts and per-candidate reasons.
```

R.15 (idempotency):

```bash
cargo run -p daemon -- --ingest net-gate  # populates
ROW_COUNT_BEFORE=$(sqlite3 state/state.db "SELECT COUNT(*) FROM candidate_net_margin_assessments;")
cargo run -p daemon -- --ingest net-gate  # should be no-op
ROW_COUNT_AFTER=$(sqlite3 state/state.db "SELECT COUNT(*) FROM candidate_net_margin_assessments;")
echo "before=$ROW_COUNT_BEFORE after=$ROW_COUNT_AFTER (must be equal)"
# Also: log line should report skipped_unchanged=R.1_count
```

---

## §8 — FAILURE-MODE PLAYBOOK

| Symptom | Likely cause | Remediation |
|---|---|---|
| Aggregates lands as PASS at 9999 bps | D5 not firing for the NULL `order_quantity_basis` branch | Re-read D5 second row. The Aggregates row has no `order_quantity_basis` in `confidence_breakdown` (verified at handoff time). The NULL fallback must check `revenue_basis_type='expected_value'` AND `supplier_quotes.price_basis_qty` IS NOT NULL → emit `quantity_basis_undetermined` |
| All 3 carpet rows lack `awaiting_rfq_quote` reason | D3 not applied OR margin verdict computed despite rfq_required | When `revenue_basis_type='rfq_required'`, skip margin-based verdict computation entirely; force REVIEW with `awaiting_rfq_quote`. The margin band can still be recorded as observed (0/0/0) but is not used to derive the verdict |
| 13 unpriced-searched rows have confidence reasons stacked on top of `no_pricing_signal` | D2 NULL-handling rule incorrectly applied; D4 exception missed | For any candidate with `pricing_assessment_id IS NULL`, **reasons must be exactly `["no_pricing_signal"]`** — do not evaluate confidence floors. See D2 final paragraph |
| Migration 0007 not applied → first INSERT fails with "no such table" | Forgot §2.2 registration steps (constant + MIGRATIONS array + DEFAULT_SCHEMA_EPOCH bump) | Re-read §2.2. All three steps required |
| `pipeline_assessments` has 0 rows but candidate rows exist | Rollup logic gated on PASS-only candidates | Rollup runs whenever any candidate row exists for a process (even all-REVIEW). See D6 first paragraph |
| `unresolved_mandatory_lots` always 0 even when lots have only `no_pricing_signal` candidates | "PASS-or-REVIEW priced candidate" misinterpreted to include unpriced REVIEWs | `no_pricing_signal` is REVIEW but **not priced**. The "priced" qualifier excludes them. See D6 unresolved_mandatory_lots row |
| Re-running mutates verdicts non-deterministically | thresholds_version not pinned, OR basis_hash composition order varies | thresholds_version must be a constant `'v1.0-2026-05-01'` at the top of the Stage 12 module. basis_hash inputs must be in the exact order shown in §2.3, with `:` separators |
| Idempotent run still rewrites rows | Skip-on-match check missing or comparing wrong field | After computing the new basis_hash, query the persisted row's basis_hash; if match, return early WITHOUT calling UPSERT. The summary line must reflect this in `skipped_unchanged=N` |
| Quality gates fail in `pdf_structural_extraction` | Docling not installed locally | These tests auto-skip per cleanup commit `7b77e94`. If they're not skipping, the `docling_available()` helper regressed; fix that, do not bypass the gate |
| `schema_activation` test fails after registering 0007 | Schema epoch mismatch | Per §2.2: epoch was 3 before; should now be 4. Update both the MIGRATIONS entry AND `DEFAULT_SCHEMA_EPOCH`. The schema_activation test in §5 verifies upgrade-path correctness |
| Found yourself modifying `crates/daemon/src/jobs/`, `crates/daemon/src/agents/`, `skills/landed-cost/`, or any Stage 11 code | Out of scope — Stage 12 is read-mostly + write-to-two-tables | Stop. Re-read §9 constraints |

---

## §9 — CONSTRAINTS

1. **Do not modify Stage 11 code.** No changes under `run_landed_cost_*`, no changes to skill prompts under `skills/landed-cost/`. Stage 12 is a pure consumer of Stage 11 output.
2. **Do not consume the operator-override mechanism.** Stage 14 will introduce it. Stage 12 v1 has no override path. The `re_evaluation_eligible` flag is a marker for Stage 14 to know which rows can be re-evaluated; Stage 12 never reads it.
3. **Do not wire `--ingest net-gate` into `run_deterministic_pipeline`.** Standalone trigger only. Pipeline integration is a follow-up bead.
4. **No skill changes.** Stage 12 has no skill component. If you find yourself editing `skills/`, stop.
5. **No silent test exclusion.** `cargo test -p daemon --bin daemon` is necessary but not sufficient (per AGENTS.md `### Stage Validation Test Gate`). The per-file integration loop is mandatory.
6. **Migration is `IF NOT EXISTS` and idempotent.** A second daemon startup must be a no-op for the schema change.
7. **Do not modify `pipeline_assessments` schema.** It already has every column Stage 12 needs (verified at handoff time). New columns are out of scope.
8. **Reasons array is JSON `string[]`, not `object[]`.** Closed enum (R.3). Stage 13 will key UI strings off these codes; do not use freeform messages.
9. **Test additions live next to existing patterns.** Unit tests in `crates/daemon/src/main.rs` (`mod tests`); migration test in `crates/daemon/tests/schema_activation.rs`. No new test crates.

---

## §10 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/stage-12-validation-manifest.md`. Use the template below **verbatim** (fill in the placeholders); do not invent your own format. Mirrors `docs/pipeline/stage-11-real-data-validation-manifest.md`.

Sections:

```markdown
# Stage 12 Net Margin Gate Validation Manifest

> Generated: {ISO timestamp}
> Status: {GREEN | RED}

## Summary
{1 paragraph: what was built, what the live DB looks like after,
verdict distribution, coverage rollup observations.}

## Build Summary
- Migration: `migrations/0007_net_margin_gate_assessments.sql`
- Migration registered: `crates/daemon/src/db/sqlite.rs` MIGRATIONS array (epoch 4)
- Code: `run_net_margin_gate_live_loop` at `main.rs:LINE`
- Dispatch: `--ingest net-gate` at `main.rs:LINE`
- New unit tests: T.1–T.18 (all in `mod tests`)
- New schema test: `ngg_t_schema_activation_007_*` in `crates/daemon/tests/schema_activation.rs`

## Live Run Result
{Final daemon log line in fenced block}

## Per-Candidate Verdict Distribution
{Table: verdict, count, sample reasons}

## Per-Process Rollup
{Table: process_id, pipeline_state, review_required, coverage metrics}

## R.1–R.18 Acceptance Criteria
| # | Criterion | Result | Detail |

## Quality Gates
- cargo fmt --check: {PASS/FAIL}
- cargo check --all-targets: {PASS/FAIL}
- cargo clippy --all-targets -- -D warnings: {PASS/FAIL}
- cargo test -p daemon --bin daemon: {PASS/FAIL — N tests, includes T.1–T.18}
- per-file integration loop: {PASS/FAIL — 39 files}

## Issues Found & Fixed
{Each: symptom → root cause → fix → verification}

## Code Changes
| File | Change |

## Out-of-Scope (Documented, Not Fixed Here)
- Operator override mechanism (Stage 14)
- Wiring `--ingest net-gate` into `run_deterministic_pipeline`
- Lot-level `mandatory` flag (no schema field today)
```

### Quality gates (the broader gate, mandatory per AGENTS.md)

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

If any single integration test target fails, fix the root cause. Do **not** weaken the test or use `#[ignore]` to bypass it. The cleanup commit `7b77e94` already auto-skips environment-dependent tests (Docling) — if Docling tests are now failing, the auto-skip helper regressed; fix that.

---

## §11 — REMINDERS

- ULTRATHINK and be meticulous.
- The expected outcome on today's data is **0 PASS, 17 REVIEW, 0 REJECT**. That is correct, not a bug. The pipeline genuinely needs operator input (quantity overrides, RFQ quotes, manual pricing) before any candidate can auto-pass. **Do not engineer a PASS to feel good.**
- Aggregates is the most interesting test case: high gross margin (9999 bps) and confidences that mostly clear floors, but quantity-basis-undetermined demotes it to REVIEW. If your implementation lets Aggregates land as PASS, you've missed D5.
- The 13 unpriced-but-searched candidates are the operator-visibility win — they used to be invisible (no Stage 10/11 row). After Stage 12 they appear as REVIEW with `no_pricing_signal`. Verify all 13 surface.
- T.1–T.18 are not optional. Stage 11 hardening shipped 5 untested code paths; the integration test cleanup just spent a round of work cleaning that up. Do not repeat the pattern.
- If you find a design corner that D1–D6 genuinely don't cover, **ask before guessing**. Better to pause than to ship a verdict policy nobody signed off on.
- Once the manifest is GREEN, stop. Stage 13 (Manifest generation) is the next stage; it is a separate handoff.
