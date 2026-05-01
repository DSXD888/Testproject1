# Stage 14 — Operator Decision (Phase 1: CLI capture path) — Build, Validate, Test, Fix in Loop

ULTRATHINK and be meticulous.

## MISSION

Stage 14 is the operator's daily-driver surface. The operator opens a Stage 13 manifest (today: 7 manifests, all `stoplight=clarify`, 18 clarification prompts across 16 candidates), answers clarification prompts where applicable, optionally re-runs the upstream pipeline (Stages 11→12→13) to refresh the manifest, then captures a final disposition (`approve` / `reject` / `watch`) into `pending_operator_intents`.

Phase 1 of Stage 14 ships the **CLI capture path** — operator types commands, the daemon writes records. Phase 2 (separate later handoff) builds the Tauri desktop UI on top of the same data plumbing. **Do not build the desktop UI in Phase 1.**

You are not just building it. You will **build → validate → test → fix in a loop** until every numbered acceptance criterion (R.1–R.16 below) is GREEN with an end-to-end demo run that proves: clarify a DGU candidate → re-run pipeline → manifest refreshes → operator decides → `pending_operator_intents` row exists with correct hashes. Do not stop with any criterion failing. Do not silently weaken the test gate to make it green.

```
┌──────────────────────────────────────────────────────────────────┐
│                          THE LOOP                                  │
│                                                                    │
│   1. Pre-flight (verify the facts in §0 below)                    │
│   2. Implement: migration 0009 + register + CLI subcommands +     │
│      override-injection in Stages 10/11 + pending_operator_intents│
│      write path + stale detection + tests                          │
│   3. Run end-to-end demo per §3                                   │
│   4. Validate against R.1..R.16 (queries inline below)            │
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

These facts were confirmed against the live tree at handoff authoring time.

| Claim | Verify command |
|---|---|
| `pending_operator_intents` table EXISTS in migration `0004_l2_pending_operator_intents.sql`. Schema: `id`, `process_id`, `desired_disposition`, `reviewed_basis_hash`, `reviewed_manifest_hash`, `reviewed_at_stage_run_id`, `reviewed_at`, `base_remote_revision`, `idempotency_key UNIQUE`, `status` (default `queued`), `superseded_by_intent_id`, `queued_at`, `decision_note`, `last_attempt_at`, `last_error`. **`desired_disposition` has a `CHECK` constraint locking to `'approve' / 'reject' / 'watch'`** — Stage 14 ships with 3-state. Adding `'hold'` would require a rewrite migration; out of scope. `status` is freely-typed (no CHECK), so transitions are flexible | `sqlite3 state/state.db ".schema pending_operator_intents"` |
| `DEFAULT_SCHEMA_EPOCH = 5`; migrations run through `0008_manifest_generation_columns.sql`. **Stage 14 adds `migrations/0009_operator_input_overrides.sql` and bumps epoch to 6** | `grep -n "DEFAULT_SCHEMA_EPOCH" crates/daemon/src/db/sqlite.rs` |
| `pdf_t_schema_activation_009_default_schema_epoch_is_5` at `crates/daemon/tests/schema_activation.rs:1170` asserts `DEFAULT_SCHEMA_EPOCH == 5` AND `user_version == 5`. **This test WILL FAIL when you bump epoch to 6 — rename to `_is_6` or add a new test (same gotcha as Stages 12 and 13)** | `grep -n "default_schema_epoch_is_" crates/daemon/tests/schema_activation.rs` |
| `infer_order_quantity_basis(row: &LandedCostAssessmentRow) -> OrderQuantityBasis` at `crates/daemon/src/main.rs:3449` resolves the existing 4 variants: `FromContractValue { qty, contract_value_gbp_pence, basis_unit }`, `FromLotQuantity { qty, basis_unit }`, `RfqRequired { fallback_qty, basis_unit }`, `SupplierCapacity { qty, basis_unit }`. **Stage 14 adds a 5th variant `FromOperatorOverride { qty: f64, basis_unit: Option<String> }`**. Adding a new enum variant means **EVERY existing `match` arm over `OrderQuantityBasis` becomes a Rust compile error** until updated — there are 17 callers; expect to touch test fixtures too. Verify after each edit with `cargo check -p daemon` | `grep -B2 -A20 "^enum OrderQuantityBasis\b" crates/daemon/src/main.rs` |
| **Override-injection design (cleaner than signature-changing the helper)**: keep `infer_order_quantity_basis(row)` PURE. Instead, extend `LandedCostAssessmentRow` with a new optional field `operator_override: Option<OperatorQuantityOverride>` and populate it at load time in `load_landed_cost_assessments(...)` via a new `read_active_quantity_override(conn, process_id, candidate_id)` helper. The pure-function then becomes `if let Some(ov) = &row.operator_override { return OrderQuantityBasis::FromOperatorOverride { qty: ov.qty, basis_unit: Some(ov.unit.clone()) }; }` followed by the existing inference. This isolates DB lookup to the loader and keeps the 17 existing pure-function call sites unchanged | inline pattern in §2.5 |
| Stage 10's revenue resolution lives in **`allocate_gross_margin_revenue(tx: &Transaction<'_>, candidate: &GrossMarginCandidateRow) -> Result<RevenueAllocation>` at `main.rs:17687`**, called from `run_gross_margin_gate_stage` at `main.rs:17778`. The function **already takes `tx: &Transaction<'_>`** (not `&Connection`); the override-read helper Stage 14 adds (`read_active_rfq_override(tx, ...)`) should match this signature. `Transaction` derefs to `Connection`, so `tx.query_row(...)` patterns work | `grep -B2 -A15 "^fn allocate_gross_margin_revenue" crates/daemon/src/main.rs` |
| **`OrderQuantityBasis::kind() -> &'static str`** is the existing manual JSON-key builder (returns `"from_contract_value"`, `"from_lot_quantity"`, `"rfq_required"`, `"supplier_capacity"`). Stage 14 adds one line: `Self::FromOperatorOverride { .. } => "from_operator_override"`. Cleaner than I implied earlier — there's no separate JSON builder hidden away; it's a one-line `impl` extension | `grep -B2 -A12 "impl OrderQuantityBasis" crates/daemon/src/main.rs` |
| **`extract_manifest_frontmatter` already exists** at `main.rs:7971` (it was added during the Option B Stage 13 build, despite my earlier round notes saying it didn't). Stage 14 doesn't actually need it (Stage 14 reads `manifests.manifest_basis_hash` directly from the DB column, not from frontmatter parsing). Mentioned only so the agent doesn't accidentally re-implement it | `grep -n "fn extract_manifest_frontmatter" crates/daemon/src/main.rs` |
| **`load_landed_cost_assessments` at `main.rs:3804`** is where `LandedCostAssessmentRow` instances are built from SQL queries. **This is where the override field gets populated** — modify the loader to call `read_active_quantity_override(tx_or_conn, process_id, candidate_id)` for each row | `grep -n "fn load_landed_cost_assessments" crates/daemon/src/main.rs` |
| **`no_pricing_signal` overrides are RECORDED ONLY in Phase 1** — they do NOT auto-synthesize a `candidate_pricing_assessments` row. The override row in `operator_input_overrides` captures the operator's intent (manual_price / un_pursueable / defer) and is shown in the desktop UI later (Phase 2). The operator can capture a decision on the still-unpriced manifest using their own knowledge of the manual price. Auto-synthesis from manual_price overrides is deferred to a follow-up bead because injecting synthetic pricing rows into Stage 10's flow is non-trivial (Stage 10 normally only processes candidates with `supplier_quotes`) | inline analysis in §2.5 |
| Existing CLI dispatch pattern: `--ingest <kind>` match arms at `crates/daemon/src/main.rs:1174-1188` (`fts`, `contracts-finder`, `landed-cost`, `net-gate`, `manifest`). **Stage 14 adds NEW top-level CLI flags** (not new `--ingest` arms): `--capture-decision`, `--capture-clarification`, `--withdraw-clarification`, `--withdraw-decision`, `--list-decisions`. Operator actions are not "ingest" actions | `grep -n '"net-gate" =>' crates/daemon/src/main.rs` |
| `Cli` struct uses clap derive `Parser`. NEW subcommands go into a `Subcommand` enum or new `#[arg]` fields on Cli. `Cli` does NOT derive `Clone` (verified in Stage 13 audits) — not a concern for Stage 14 since we don't need concurrent skill invocation | `grep -B5 "^struct Cli\b" crates/daemon/src/main.rs` |
| `persist_stage(tx, process_id, stage, scope_kind, scope_id, stage_run_id)` at `main.rs:14196+` is the canonical stage-tracking helper. Stage 14 uses it with stage=`'operator decision'`, scope_kind=`'process'` | `grep -n "fn persist_stage(" crates/daemon/src/main.rs` |
| `allocate_stage_run_id()` at `crates/daemon/src/jobs/runner.rs:728` returns a fresh stage_run_id. Use one per CLI invocation that mutates state | `grep -n "fn allocate_stage_run_id" crates/daemon/src/jobs/runner.rs` |
| Today's data: 7 manifests all `stoplight='clarify'`; 18 clarification prompts; DGU process (`proc-pc-7046e8ec-...`) is the most-likely-pursuable import case (HMRC `7008002000`, real Alibaba quote, real SeaRates freight) | `sqlite3 state/state.db "SELECT process_id, stoplight, json_array_length(clarification_prompts_json) FROM manifests"` |
| Tauri desktop app exists at `apps/desktop/src-tauri/` + `apps/desktop/src/`. **Phase 1 leaves it alone** — Phase 2 (later handoff) wires the desktop UI on top of the CLI's data plumbing | `ls apps/desktop/` |
| `agent-mail` MCP server is healthy (`mcp__agent-mail__health_check` returns `green`); commits require `AGENT_NAME` env var (e.g., `AGENT_NAME=claude-code git commit ...`). Stage 13 work was committed this way | inline check during commit phase |

If any drift, the rest of this handoff still holds — just update the matching numbers in the validation queries.

---

## §1 — DECISIONS ALREADY MADE (DO NOT RE-OPEN)

The operator has signed off on D1–D6. Implement to spec; do not deviate.

### D1 — Disposition vocabulary: 3-state (`approve` / `reject` / `watch`)

| Disposition | Semantic |
|---|---|
| `approve` | Pursue this opportunity (Stage 15 / bid_workflow may eventually act on this; today it's just a record) |
| `reject` | Drop the opportunity; no further pipeline work needed |
| `watch` | Monitor for changes — re-render manifest if Stage 12 outputs change; serves dual purpose as "I'll come back later" (defer). The basis_hash drift detection (D3) handles the staleness signal automatically |

**Why 3-state**: matches the existing `CHECK` constraint on `pending_operator_intents.desired_disposition`. Adding `'hold'` would require a SQLite table rewrite migration (drop CHECK, copy table, re-add) — out of scope for this stage. `watch` covers both monitor-for-changes and defer-decision use cases cleanly.

### D2 — Clarification flow: inline overrides + persistent + manual re-run

When an operator wants to act on a clarification prompt:

1. **Operator invokes** `--capture-clarification --process-id <pid> --candidate-id <cid> --reason-code <code> --payload <json>`. The daemon writes to a NEW table `operator_input_overrides` (per §2.1).
2. **Operator invokes the upstream chain manually**: `--ingest landed-cost && --ingest net-gate && --ingest manifest`. The daemon, upon re-running, **reads the override table and injects values into Stages 10/11** (per §2.5).
3. **Operator opens the refreshed manifest** and either captures another clarification or proceeds to disposition.

**Persistence model**: overrides are persistent until explicitly withdrawn. One row per `(process_id, candidate_id, reason_code)` tuple — re-providing the same clarification UPDATES (no duplicate row). Withdrawing creates a `withdrawn_at` timestamp without deleting the row (audit trail).

**Freshness signal**: each override records `basis_hash_when_provided` (the manifest's basis_hash at the time the operator answered). When the manifest's basis_hash later differs, the desktop UI (Phase 2) can show "this override was provided against an earlier version of the manifest" — Phase 1 just records the field; Phase 2 surfaces it.

**Why manual re-run rather than auto-trigger**: lets operator batch multiple clarifications before re-running; transparent control; matches the `--ingest` pattern of explicit pipeline triggers.

### D3 — Re-decision policy: mutable with audit chain + soft auto-invalidate

| Scenario | Behavior |
|---|---|
| Operator captures a fresh decision | INSERT new `pending_operator_intents` row, `status='queued'`, `superseded_by_intent_id=NULL` |
| Operator changes their mind on the same process | INSERT a new row; on the OLD row, set `superseded_by_intent_id` to the new row's id and `status='superseded'`. The latest-non-superseded view is the canonical "current decision" (existing index `idx_pending_operator_intents_latest` already supports this) |
| Stage 13 re-runs and produces a new `manifest_basis_hash` | The daemon detects this when Stage 13 writes a new manifest row whose `manifest_basis_hash` differs from any existing intents' `reviewed_basis_hash`. Mark those intents `status='stale'`. The intent is still queryable; the desktop UI prompts for re-confirm before any execution |
| Future Stage 15 (bid_workflow) wants to act | Requires `status='queued'` (active and current). `'stale'`, `'superseded'`, `'withdrawn'` all block execution. Out of scope for Stage 14 itself but the handoff respects the contract |

**Stale detection trigger**: when Stage 13 writes a manifest row, ALSO scan `pending_operator_intents` for matching `process_id`. Any intent whose `reviewed_basis_hash != new_manifest.manifest_basis_hash` AND whose `status='queued'` transitions to `status='stale'`. This is a small SQL UPDATE; can run inside Stage 13's existing transaction or as a follow-up step.

### D4 — Evidence presentation at decision time: kitchen-sink (Phase 2 concern)

In Phase 2 the desktop UI shows: rendered markdown manifest + Quick-Decide panel (stoplight badge + disposition buttons + reasoning_text input) + drill-downs into per-candidate detail + inline clarification form. **Phase 1 ships only the CLI**. The data plumbing supports Phase 2 (everything is in the DB and on disk; the UI is presentation only).

For Phase 1, the operator reads the markdown manifest with their preferred tool (`cat artifacts/manifests/{pid}/manifest.md`, or open in their editor) and captures decisions via CLI. The CLI prints a JSON summary of what was written so the operator can verify.

### D5 — Phase split: CLI in Phase 1, Desktop in Phase 2 (separate handoff)

This handoff covers Phase 1 only. Phase 2 (Tauri desktop UI) is a future handoff that reuses the same data plumbing. **Do NOT touch `apps/desktop/`** in Phase 1 except possibly to confirm it still builds.

### D6 — Audit metadata: optional `decision_note` (= reasoning_text)

The existing schema has a `decision_note TEXT` (NULL-able) column. Stage 14 uses this for the operator's optional free-form reasoning. The CLI accepts `--reasoning "<text>"` and writes it to `decision_note`. No closed-enum tags in v1; can add later when there's enough decision history to make tags useful.

The `idempotency_key` is REQUIRED (it's a UNIQUE column). The CLI accepts `--idempotency-key <key>` and rejects duplicates. This protects against double-submit if the operator re-runs the same command.

---

## §2 — ARCHITECTURE

### §2.1 — New migration: `migrations/0009_operator_input_overrides.sql`

**No `BEGIN;` / `COMMIT;` in the file** — `apply_sql_file` already wraps in `SAVEPOINT state_db_migration` (same lesson learned from Stage 12 and Stage 13). Bare DDL only.

```sql
CREATE TABLE IF NOT EXISTS operator_input_overrides (
  id TEXT PRIMARY KEY,                                  -- 'override-{process_id}-{candidate_id}-{reason_code}'
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  product_candidate_id TEXT NOT NULL REFERENCES product_candidates(id),
  reason_code TEXT NOT NULL,                            -- which clarification reason this answers
  payload_json TEXT NOT NULL,                           -- structured operator answer (qty, RFQ details, etc.)
  notes TEXT,                                           -- optional operator note
  basis_hash_when_provided TEXT,                        -- manifest basis_hash visible to operator at provide-time
  provided_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  provided_by TEXT NOT NULL DEFAULT 'local-operator',
  withdrawn_at TEXT,                                    -- soft-delete timestamp (NULL when active)
  superseded_by_override_id TEXT REFERENCES operator_input_overrides(id),
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(process_id, product_candidate_id, reason_code)
);

CREATE INDEX IF NOT EXISTS idx_oio_active
  ON operator_input_overrides(process_id, product_candidate_id, reason_code)
  WHERE withdrawn_at IS NULL AND superseded_by_override_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_oio_process ON operator_input_overrides(process_id);
```

**Reason-code enum** (for `reason_code` column — not enforced by CHECK, but enforced in code):
- `quantity_basis_undetermined` (operator provides order quantity)
- `quantity_basis_unaligned` (operator provides actual order qty when Stage 11 inferred capacity)
- `awaiting_rfq_quote` (operator provides RFQ quote: per-unit price + lead time + MOQ + currency)
- `no_pricing_signal` (operator provides manual price OR flags un-pursueable)

**Payload-json schema** (one shape per reason_code; the CLI validates before writing):

```json
// quantity_basis_undetermined / quantity_basis_unaligned
{ "qty": 100.0, "unit": "square meter" }

// awaiting_rfq_quote
{ "unit_price_gbp_pence": 4500, "lead_time_days": 21, "moq": 50, "currency": "GBP", "valid_until": "2026-06-01" }

// no_pricing_signal — operator chooses one variant:
{ "variant": "manual_price", "unit_price_gbp_pence": 1200, "unit": "each", "source_note": "buyer-supplied list 2026-04" }
{ "variant": "un_pursueable", "reason": "no UK supplier; CN-only with import controls" }
{ "variant": "defer", "reason": "wait for Stage 9 to find more candidates" }
```

### §2.2 — Migration registration (REQUIRED, easy to forget)

In `crates/daemon/src/db/sqlite.rs`:

1. Add a constant near `L_MANIFEST_MIGRATION_PATH`:
   ```rust
   const L_OPERATOR_OVERRIDES_MIGRATION_PATH: &str = "migrations/0009_operator_input_overrides.sql";
   ```
2. Add an entry to the `MIGRATIONS` array (after `0008`):
   ```rust
   ("0009_operator_input_overrides", 6, L_OPERATOR_OVERRIDES_MIGRATION_PATH),
   ```
3. Update `pub const DEFAULT_SCHEMA_EPOCH: i64` to `6`.
4. **Update or rename `pdf_t_schema_activation_009_default_schema_epoch_is_5`** at `crates/daemon/tests/schema_activation.rs:1170`. Either rename to `_is_6` (asserting `== 6`) OR add a new test alongside it. Same gotcha as Stages 12 and 13.

### §2.3 — New schema-activation test

Add to `crates/daemon/tests/schema_activation.rs` mirroring `mfg_t_schema_activation_008_upgrade_path_applies_0008_cleanly`:

```rust
#[test]
fn opd_t_schema_activation_009_upgrade_path_applies_0009_cleanly() -> Result<()> {
    // Apply 0001..0008 manually; seed schema_migrations to epoch 5;
    // open StateDb (which should apply 0009 + bump epoch to 6);
    // assert operator_input_overrides table exists with the expected columns;
    // assert schema_migrations row for '0009_operator_input_overrides' exists.
}
```

### §2.4 — CLI subcommand additions

Add four top-level `--capture-*` / `--withdraw-*` / `--list-*` flags to the `Cli` struct (clap derive). The CLI dispatch in main.rs gets a NEW match block that runs BEFORE the existing `--ingest` dispatch. These are mutually exclusive with `--ingest` (operator action vs pipeline trigger).

**Sketch** (the agent picks the exact clap shape — sub-command pattern OR top-level flags is the agent's call; the existing CLI uses top-level flags so I lean that way):

```rust
// New fields on Cli (clap derive Parser):
#[arg(long, conflicts_with_all = ["ingest", "capture_clarification", ...])]
capture_decision: Option<String>,           // process_id
#[arg(long, requires = "capture_decision")]
disposition: Option<String>,                // approve|reject|watch
#[arg(long, requires = "capture_decision")]
idempotency_key: Option<String>,
#[arg(long)]
reasoning: Option<String>,

#[arg(long)]
capture_clarification: Option<String>,      // process_id
#[arg(long, requires = "capture_clarification")]
candidate_id: Option<String>,
#[arg(long, requires = "capture_clarification")]
reason_code: Option<String>,
#[arg(long, requires = "capture_clarification")]
payload: Option<String>,                    // JSON string
#[arg(long)]
note: Option<String>,

#[arg(long)]
withdraw_clarification: Option<String>,     // process_id; requires --candidate-id + --reason-code
#[arg(long)]
withdraw_decision: Option<String>,          // idempotency_key

#[arg(long)]
list_decisions: bool,                       // optionally --process-id <pid> + --status <s>
#[arg(long)]
list_status_filter: Option<String>,
```

**Dispatch order in main.rs `main()`**: check `cli.capture_decision.is_some() || cli.capture_clarification.is_some() || cli.withdraw_clarification.is_some() || cli.withdraw_decision.is_some() || cli.list_decisions` BEFORE the `--ingest` match. These actions don't need the live-loop runtime initialization beyond opening `state_db`. Dispatch and `return Ok(())` from the handler — never fall through into `--ingest` after handling an operator action. clap's `conflicts_with_all` should also prevent both being passed simultaneously, but the dispatch ordering is the defensive belt-and-braces.

### §2.5 — Override-injection into Stages 10 and 11

Two injection points for the four override reason codes:

**`quantity_basis_undetermined` and `quantity_basis_unaligned` → Stage 11 (via row-extension)**

Keep `infer_order_quantity_basis(row: &LandedCostAssessmentRow) -> OrderQuantityBasis` pure. Instead:

1. Add a new field to `LandedCostAssessmentRow`:
   ```rust
   struct LandedCostAssessmentRow {
       // ... existing fields ...
       operator_override: Option<OperatorQuantityOverride>,   // NEW
   }
   struct OperatorQuantityOverride {
       qty: f64,
       unit: String,
   }
   ```
2. In `load_landed_cost_assessments` (or whichever query builds the row), populate `operator_override` via a new helper:
   ```rust
   fn read_active_quantity_override(conn: &Connection, process_id: &str, candidate_id: &str)
     -> Result<Option<OperatorQuantityOverride>>
   {
       // SELECT payload_json FROM operator_input_overrides
       //   WHERE process_id = ?1 AND product_candidate_id = ?2
       //     AND reason_code IN ('quantity_basis_undetermined', 'quantity_basis_unaligned')
       //     AND withdrawn_at IS NULL AND superseded_by_override_id IS NULL
       //   ORDER BY provided_at DESC LIMIT 1
       // Parse payload_json as { qty, unit }
   }
   ```
3. Add a 5th variant to `OrderQuantityBasis`:
   ```rust
   FromOperatorOverride { qty: f64, basis_unit: Option<String> }
   ```
4. Modify `infer_order_quantity_basis` (the pure function — signature UNCHANGED) to short-circuit on the new field:
   ```rust
   fn infer_order_quantity_basis(row: &LandedCostAssessmentRow) -> OrderQuantityBasis {
       if let Some(ov) = &row.operator_override {
           return OrderQuantityBasis::FromOperatorOverride {
               qty: ov.qty,
               basis_unit: Some(ov.unit.clone()),
           };
       }
       // existing inference logic continues unchanged
   }
   ```
5. **Update every existing `match OrderQuantityBasis` site** to handle `FromOperatorOverride` — Rust will compile-error on each unhandled match. Most match arms can treat it identically to `FromContractValue` (a fully-trusted explicit quantity). The 17 callers I counted are mostly tests; production sites are 2 (line 3500 and 5477).

**Serde wire format**: `OrderQuantityBasis` doesn't auto-derive Serialize today (the `confidence_breakdown.order_quantity_basis` JSON shape `{kind, qty, source, unit}` is built manually). Find that builder; add a case for `FromOperatorOverride` that emits `kind: "from_operator_override"`, `qty`, `source: "operator_input_overrides"`, `unit`. Stage 14's R.9 query checks for `kind="from_operator_override"`.

**`awaiting_rfq_quote` → Stage 10 `allocate_gross_margin_revenue`**

The function exists at `main.rs:17687` with signature `fn allocate_gross_margin_revenue(tx: &Transaction<'_>, candidate: &GrossMarginCandidateRow) -> Result<RevenueAllocation>`. Add the override check at the top:

```rust
fn allocate_gross_margin_revenue(
    tx: &Transaction<'_>,                          // EXISTING signature; do not change
    candidate: &GrossMarginCandidateRow,
) -> Result<RevenueAllocation> {
    // NEW: check for an active RFQ override
    if let Some(rfq) = read_active_rfq_override(tx, &candidate.process_id, &candidate.product_candidate_id)? {
        let qty = read_active_quantity_override(tx, &candidate.process_id, &candidate.product_candidate_id)?
            .map(|q| q.qty)
            .unwrap_or(rfq.moq as f64);   // fall back to MOQ if operator hasn't given a separate qty
        let revenue = (rfq.unit_price_gbp_pence as f64 * qty).round() as i64;
        return Ok(RevenueAllocation {
            revenue_basis_type: "expected_value".to_owned(),
            expected_addressable_revenue_ex_vat_pence: revenue,
            revenue_confidence_bps: 8000,    // operator-supplied → high confidence
            // ... preserve any other RevenueAllocation fields with sensible defaults
        });
    }
    // existing path continues unchanged
}
```

`read_active_rfq_override(tx, ...)` is the analogue of `read_active_quantity_override(tx, ...)` but parses `{ unit_price_gbp_pence, lead_time_days, moq, currency }`. Both helpers take `&Transaction<'_>` (or `&Connection` — `Transaction` derefs cleanly).

**Phase 1 assumption**: `currency = GBP`. Reject non-GBP RFQ payloads at `--capture-clarification` time with a clear error. Cross-currency RFQ overrides need FX skill integration (out of scope).

**`no_pricing_signal` → RECORDED ONLY (no auto-synthesis in Phase 1)**

Operator captures the override (variant: `manual_price` / `un_pursueable` / `defer`), the row lands in `operator_input_overrides`, and that's it for Phase 1. **No injection into Stage 10 or Stage 11.** The candidate continues to manifest as `no_pricing_signal` REVIEW; the operator captures a decision based on their own knowledge of the manual price.

Auto-synthesis from manual_price overrides is deferred to a follow-up bead because Stage 10's flow is built around `supplier_quotes` and injecting a synthetic pricing path is non-trivial. Recording the override Phase-1 keeps the operator action durable; Stage 15 (or future Stage 14b) can read it.

### §2.6 — `pending_operator_intents` write path

CLI command: `--capture-decision --process-id <pid> --disposition <d> --idempotency-key <key> [--reasoning <text>]`.

Daemon flow:
1. Validate disposition is in `('approve', 'reject', 'watch')` — fail with clear error if not (matches CHECK constraint)
2. Look up the latest `manifests` row for the `process_id` — capture `manifest_basis_hash` and `artifact_hash`
3. Look up the latest `pipeline_assessments.decision_basis_hash` for the same process — capture as `reviewed_basis_hash`
4. Reject if there's no manifest (no decision is meaningful without one)
5. Build a new `pending_operator_intents` row:
   - `id = Uuid::new_v4().to_string()` — fresh UUID. Idempotency_key has its own UNIQUE constraint and handles dedup; deriving id from idempotency_key is risky because the operator-supplied key may contain characters unsafe for IDs (slashes, spaces, quotes). UUID PK is the standard pattern in this codebase
   - `process_id`
   - `desired_disposition`
   - `reviewed_basis_hash` = pipeline_assessments.decision_basis_hash (Stage 12 hash)
   - `reviewed_manifest_hash` = manifests.artifact_hash (Stage 13 manifest content hash)
   - `reviewed_at_stage_run_id` = a freshly allocated `allocate_stage_run_id()`
   - `reviewed_at` = now
   - `idempotency_key` = from CLI
   - `status` = `'queued'`
   - `queued_at` = now
   - `decision_note` = from CLI's `--reasoning`
6. INSERT — if UNIQUE constraint on `idempotency_key` fires, return the existing row (idempotent no-op)
7. If a previous intent exists for this process with `superseded_by_intent_id IS NULL` AND `status IN ('queued','stale')`, set its `superseded_by_intent_id` to the new row's id and `status='superseded'`
8. Stage tracking: `persist_stage(tx, process_id, "operator decision", "process", process_id, stage_run_id)`
9. Print JSON summary to stdout: `{"intent_id": "...", "process_id": "...", "disposition": "...", "superseded_id": "..." | null}`

### §2.7 — Stale detection (run inside `persist_manifest_generation_output`)

The function `persist_manifest_generation_output` is the SHARED persistence helper called by all 3 Stage 13 render modes (skill, daemon, auto-fallback). Place stale detection INSIDE it so all paths automatically trigger the transition. Don't put it at the live-loop level — that risks one mode forgetting to fire.

After the manifest row is written (and ideally inside the same `with_write_transaction`), run:

```sql
UPDATE pending_operator_intents
SET status = 'stale',
    last_attempt_at = CURRENT_TIMESTAMP,
    last_error = NULL
WHERE process_id = ?1
  AND status = 'queued'
  AND reviewed_basis_hash != ?2;  -- new manifest's manifest_basis_hash from the bundle
```

Where `?2` comes from `bundle.manifest_basis_hash` (Stage 12's hash that Stage 13 used to compose its own basis). Idempotent (re-runs with the same hash are no-ops). Does NOT touch `superseded` or `withdrawn` intents — those are terminal-ish.

### §2.8 — Stage tracking

For every operator action that mutates state (capture-decision, capture-clarification, withdraw-*):
- stage = `'operator decision'`
- scope_kind = `'process'`
- scope_id = `procurement_processes.id`
- state = `'succeeded'`

Use `persist_stage(...)`. The handoff doesn't track per-clarification sub-scope; one process-level head per operator action is sufficient.

### §2.9 — Withdrawal subcommands

`--withdraw-clarification --process-id <pid> --candidate-id <cid> --reason-code <code>`:
- Find active row in `operator_input_overrides`
- Set `withdrawn_at = now`
- Print JSON summary

`--withdraw-decision --idempotency-key <key>`:
- Find row by idempotency_key
- Set `status = 'withdrawn'`, set `superseded_by_intent_id = NULL` (no successor; explicitly retracted)
- The row stays in the table for audit; the latest-non-superseded view excludes it because of the WHERE clause on the index
- Print JSON summary

### §2.10 — `--list-decisions` (read-only inspection)

`--list-decisions [--process-id <pid>] [--list-status-filter <status>]`:

```sql
SELECT id, process_id, desired_disposition, status, idempotency_key,
       reviewed_at, decision_note
FROM pending_operator_intents
WHERE (?1 IS NULL OR process_id = ?1)
  AND (?2 IS NULL OR status = ?2)
ORDER BY queued_at DESC
LIMIT 100;
```

Emit as JSON or pretty table; agent's choice. JSON is more scriptable.

---

## §3 — END-TO-END DEMO (the validation manifest must record this)

Walk through this exact sequence as part of the validation. It proves Stage 14 actually does what it claims.

```bash
# 1. Snapshot baseline
sqlite3 state/state.db "SELECT COUNT(*) FROM pending_operator_intents;"
sqlite3 state/state.db "SELECT COUNT(*) FROM operator_input_overrides;"
sqlite3 state/state.db "SELECT manifest_basis_hash FROM manifests WHERE process_id = 'proc-pc-7046e8ec-2734-ed11-8119-005056b64545';"
# Note the DGU manifest's basis_hash for later

# 2. Operator captures a quantity clarification on the DGU
cargo run -p daemon -- \
  --capture-clarification proc-pc-7046e8ec-2734-ed11-8119-005056b64545 \
  --candidate-id pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279 \
  --reason-code quantity_basis_unaligned \
  --payload '{"qty": 100.0, "unit": "square meter"}' \
  --note "Buyer confirmed 100 m² order in email 2026-05-15"

# 3. Re-run pipeline to refresh the manifest
cargo run -p daemon -- --ingest landed-cost
cargo run -p daemon -- --ingest net-gate
cargo run -p daemon -- --ingest manifest

# 4. Verify: DGU's new manifest has a different basis_hash AND uses qty=100, not the old supplier-capacity 10000
sqlite3 state/state.db "SELECT manifest_basis_hash FROM manifests WHERE process_id = 'proc-pc-7046e8ec-2734-ed11-8119-005056b64545';"
# Should differ from step 1's snapshot

sqlite3 state/state.db "SELECT json_extract(confidence_breakdown, '\$.order_quantity_basis') FROM landed_cost_estimates WHERE supplier_quote_id = 'sq-b22fcf00b7a545d699c3438c42b9027c-1';"
# Should now reflect from_operator_override with qty=100

# 5. Operator captures a decision on the refreshed manifest
cargo run -p daemon -- \
  --capture-decision proc-pc-7046e8ec-2734-ed11-8119-005056b64545 \
  --disposition approve \
  --idempotency-key dec-2026-05-01-dgu-001 \
  --reasoning "DGU now sized at 100m²; freight + customs + VAT realistic; pursue."

# 6. Verify the intent
sqlite3 -header -column state/state.db "
SELECT process_id, desired_disposition, status, reviewed_basis_hash, reviewed_manifest_hash,
       idempotency_key, decision_note
FROM pending_operator_intents WHERE idempotency_key = 'dec-2026-05-01-dgu-001';"

# 7. Idempotency: re-run the same capture-decision command → should be no-op (UNIQUE constraint hits)
cargo run -p daemon -- \
  --capture-decision proc-pc-7046e8ec-2734-ed11-8119-005056b64545 \
  --disposition approve \
  --idempotency-key dec-2026-05-01-dgu-001 \
  --reasoning "..."
sqlite3 state/state.db "SELECT COUNT(*) FROM pending_operator_intents WHERE idempotency_key = 'dec-2026-05-01-dgu-001';"
# Must equal 1

# 8. Re-decision: capture a different disposition with a NEW idempotency_key → original gets superseded
cargo run -p daemon -- \
  --capture-decision proc-pc-7046e8ec-2734-ed11-8119-005056b64545 \
  --disposition watch \
  --idempotency-key dec-2026-05-01-dgu-002 \
  --reasoning "Holding while we get a second freight quote."
sqlite3 state/state.db "SELECT id, status, superseded_by_intent_id FROM pending_operator_intents WHERE process_id = 'proc-pc-7046e8ec-2734-ed11-8119-005056b64545' ORDER BY queued_at;"
# 001 should be 'superseded' with superseded_by_intent_id pointing to 002's id; 002 should be 'queued'

# 9. Stale detection: change quantity again, re-run pipeline → existing intent becomes stale
cargo run -p daemon -- \
  --capture-clarification proc-pc-7046e8ec-2734-ed11-8119-005056b64545 \
  --candidate-id pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279 \
  --reason-code quantity_basis_unaligned \
  --payload '{"qty": 250.0, "unit": "square meter"}' \
  --note "Updated buyer estimate 2026-05-20"
cargo run -p daemon -- --ingest landed-cost
cargo run -p daemon -- --ingest net-gate
cargo run -p daemon -- --ingest manifest
sqlite3 state/state.db "SELECT idempotency_key, status FROM pending_operator_intents WHERE process_id = 'proc-pc-7046e8ec-2734-ed11-8119-005056b64545';"
# Both 001 (already superseded, stays superseded) and 002 (was queued) — 002 should be 'stale' now

# 10. Withdraw the latest decision
cargo run -p daemon -- --withdraw-decision --idempotency-key dec-2026-05-01-dgu-002
sqlite3 state/state.db "SELECT idempotency_key, status FROM pending_operator_intents WHERE idempotency_key = 'dec-2026-05-01-dgu-002';"
# Should be 'withdrawn'

# 11. List queued decisions across all processes
cargo run -p daemon -- --list-decisions --list-status-filter queued
# Expect an empty result (everything withdrawn or stale or superseded)
```

This walks through every Stage 14 action.

---

## §4 — REQUIRED NEW UNIT TESTS

Add to `crates/daemon/src/main.rs` (`mod tests`). Every test name listed must exist with `#[test]` (or `#[tokio::test]` if async needed) and pass.

| # | Test name | What it verifies |
|---|---|---|
| T.1 | `capture_decision_inserts_pending_operator_intents_row_with_correct_hashes` | Synthetic manifest + Stage 12 row → captured decision row has matching reviewed_basis_hash, reviewed_manifest_hash, status='queued' |
| T.2 | `capture_decision_rejects_invalid_disposition` | disposition='hold' → error before INSERT (3-state enforcement) |
| T.3 | `capture_decision_idempotent_on_duplicate_idempotency_key` | Same key twice → 1 row in DB; second call is no-op-with-success |
| T.4 | `re_decision_supersedes_previous_intent` | New decision with new key → old row gets `status='superseded'`, `superseded_by_intent_id` set |
| T.5 | `capture_decision_rejects_when_no_manifest_exists` | process_id with no manifests row → error (clear message) |
| T.6 | `capture_clarification_inserts_operator_input_overrides_row` | Synthetic candidate + reason → row in operator_input_overrides; basis_hash_when_provided captured |
| T.7 | `capture_clarification_replaces_existing_active_override` | Same (process, candidate, reason) twice → 1 active row (UNIQUE) with updated payload, row_version bumped |
| T.8 | `capture_clarification_validates_payload_per_reason_code` | quantity_basis_unaligned with `{"qty":100}` ✓; with `{"foo":1}` ✗ |
| T.9 | `infer_order_quantity_basis_uses_operator_override_when_present` | Active override → returns `OrderQuantityBasis::FromOperatorOverride { qty:100, unit:"square meter" }`; no override → falls through to existing inference |
| T.10 | `infer_order_quantity_basis_ignores_withdrawn_override` | withdrawn_at IS NOT NULL → falls through to existing inference |
| T.11 | `awaiting_rfq_override_synthesizes_revenue_in_allocate_gross_margin_revenue` | candidate with revenue_basis_type='rfq_required' + active RFQ override → `allocate_gross_margin_revenue` returns `revenue_basis_type='expected_value'` AND `expected_addressable_revenue_ex_vat_pence = unit_price × qty` AND `revenue_confidence_bps >= 8000` |
| T.12 | `no_pricing_signal_override_is_recorded_but_does_not_synthesize` | candidate with no pricing_assessment + override variant='manual_price' → row in `operator_input_overrides` with payload, but NO synthetic pricing_assessment is created. Candidate continues to flow through Stage 10 as `no_pricing_signal` REVIEW |
| T.13 | `no_pricing_signal_override_payload_validates_per_variant` | `{"variant":"manual_price","unit_price_gbp_pence":1200,"unit":"each"}` ✓; `{"variant":"un_pursueable","reason":"..."}` ✓; `{"variant":"defer","reason":"..."}` ✓; `{"variant":"unknown",...}` ✗; `{"variant":"manual_price"}` (missing required fields) ✗ |
| T.14 | `stale_detection_marks_intent_stale_when_manifest_basis_hash_changes` | Existing 'queued' intent + manifest re-run with new basis_hash → intent transitions to 'stale' |
| T.15 | `stale_detection_does_not_touch_superseded_or_withdrawn_intents` | superseded/withdrawn intents stay as-is even when manifest changes |
| T.16 | `withdraw_clarification_marks_withdrawn_at` | Active override → withdrawn_at set; subsequent override-read returns None |
| T.17 | `withdraw_decision_sets_status_withdrawn` | Active intent → status='withdrawn'; `--list-decisions --list-status-filter queued` excludes it |
| T.18 | `list_decisions_filters_by_process_id_and_status` | Synthetic 3 intents across 2 processes with various statuses → filter combinations return correct subsets |

T.1–T.8, T.16–T.18 are pure-function or DB-fixture tests. T.9–T.15 require synthetic Stage 11/12 fixtures — the existing test helpers (`bootstrap_manifest_render_runtime`, etc.) are good templates.

**Test count math**: 18 new tests + the schema activation test (§2.3). Current binary unit tests = 129. After Stage 14: **129 + 18 = 147 binary unit tests**. The new `opd_t_schema_activation_009_*` joins schema_activation.rs (currently 20 tests, becomes 21).

---

## §5 — ACCEPTANCE CRITERIA — R.1 to R.16

Each must PASS before declaring done. Validation queries inline in §6.

| # | Criterion |
|---|---|
| R.1 | Migration `0009_operator_input_overrides.sql` exists; registered in `MIGRATIONS` array (epoch 6); `DEFAULT_SCHEMA_EPOCH = 6`; `pdf_t_schema_activation_009_*` test renamed/replaced for `_is_6` |
| R.2 | New `operator_input_overrides` table exists with all expected columns (id, process_id, product_candidate_id, reason_code, payload_json, notes, basis_hash_when_provided, provided_at, provided_by, withdrawn_at, superseded_by_override_id, owner_id, row_version, last_local_seq, stage_run_id, created_at, updated_at) and the UNIQUE + indexes per §2.1 |
| R.3 | New schema-activation test `opd_t_schema_activation_009_upgrade_path_applies_0009_cleanly` exists and passes |
| R.4 | CLI subcommands work end-to-end per §3 demo: `--capture-decision`, `--capture-clarification`, `--withdraw-clarification`, `--withdraw-decision`, `--list-decisions` |
| R.5 | 3-state disposition enforcement: invalid values rejected with clear error before DB write |
| R.6 | Idempotency: duplicate `--idempotency-key` is a no-op-with-success (UNIQUE constraint hits, daemon returns existing row) |
| R.7 | Re-decision audit chain: new decision sets `superseded_by_intent_id` on the previous active intent for that process; previous intent's `status='superseded'` |
| R.8 | Override persistence: re-providing the same (process, candidate, reason) UPDATEs (1 active row); withdrawn overrides stay in the table with `withdrawn_at` set |
| R.9 | Override injection: Stage 11's `infer_order_quantity_basis` returns `FromOperatorOverride` when an active override exists for the candidate; falls through to existing inference when none |
| R.10 | RFQ override flow: `awaiting_rfq_quote` override is read by `allocate_gross_margin_revenue` (Stage 10); the resulting `candidate_pricing_assessments` row has `revenue_basis_type='expected_value'` AND `expected_addressable_revenue_ex_vat_pence = unit_price_gbp_pence × order_qty` (qty from a quantity override OR RFQ payload's `moq` as fallback) |
| R.11 | Manual-price override is RECORDED only — `no_pricing_signal` overrides land in `operator_input_overrides` with correct payload but DO NOT auto-synthesize pricing_assessments. The candidate continues to flow downstream as `no_pricing_signal`. (Auto-synthesis is deferred to a follow-up bead) |
| R.12 | Stale detection: when Stage 13 writes a new manifest with a different `manifest_basis_hash`, any matching active intents transition to `status='stale'` |
| R.13 | Stage tracking: `process_stage_status` and `process_stage_heads` rows for stage='operator decision', scope_kind='process', state='succeeded' (one per operator action that mutates) |
| R.14 | FK integrity: 0 orphans on operator_input_overrides → process / candidate; 0 orphans on pending_operator_intents → process; 0 dangling `superseded_by_*` references |
| R.15 | Stages 11/12/13 invariants intact when no overrides are present: DGU `landed_cost_estimates` row unchanged; 17 `candidate_net_margin_assessments` rows unchanged; 7 `manifests` rows unchanged. After §3 demo (which adds an override), the DGU's specific landed_cost row CHANGES (qty went 10000→100, freight + VAT recompute) — that's expected; verify by snapshot diff scoped to the DGU row only |
| R.16 | Quality gates: `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p daemon --bin daemon` (T.1–T.18 → expect **147 binary unit tests pass**), per-file integration loop green across all **39 files** (`schema_activation.rs` grows from 20 to 21 internal tests) |

---

## §6 — VALIDATION QUERIES

Run these after the §3 demo + after the full quality-gate sweep:

```bash
sqlite3 -header -column state/state.db "
SELECT '=== R.2 operator_input_overrides schema sanity ===';
SELECT COUNT(*) AS table_exists FROM sqlite_master WHERE type='table' AND name='operator_input_overrides';

SELECT '=== R.4/R.6 idempotency: count rows for the demo idempotency_key ===';
SELECT idempotency_key, COUNT(*) AS rows_per_key FROM pending_operator_intents
WHERE idempotency_key LIKE 'dec-2026-05-01-dgu-%' GROUP BY idempotency_key;
-- Each idempotency_key must show exactly 1 row regardless of how many times --capture-decision was invoked

SELECT '=== R.7 re-decision audit chain ===';
SELECT id, idempotency_key, status, superseded_by_intent_id
FROM pending_operator_intents
WHERE process_id = 'proc-pc-7046e8ec-2734-ed11-8119-005056b64545'
ORDER BY queued_at;
-- Expect: dgu-001 'superseded' with superseded_by_intent_id pointing to dgu-002's id; dgu-002 'stale' or 'withdrawn' depending on demo step reached

SELECT '=== R.8 override persistence: 1 active row per (proc, candidate, reason) ===';
SELECT process_id, product_candidate_id, reason_code, COUNT(*) AS active_rows
FROM operator_input_overrides
WHERE withdrawn_at IS NULL AND superseded_by_override_id IS NULL
GROUP BY process_id, product_candidate_id, reason_code
HAVING active_rows > 1;
-- Must be empty

SELECT '=== R.9 DGU now uses operator override for order quantity ===';
SELECT json_extract(confidence_breakdown, '\$.order_quantity_basis.kind') AS basis_kind,
       json_extract(confidence_breakdown, '\$.order_quantity_basis.qty') AS qty
FROM landed_cost_estimates
WHERE supplier_quote_id = 'sq-b22fcf00b7a545d699c3438c42b9027c-1';
-- After §3 step 9, basis_kind should be 'from_operator_override' and qty should be 250.0 (or whatever the most-recent demo override set)

SELECT '=== R.12 stale detection ===';
SELECT idempotency_key, status, reviewed_basis_hash
FROM pending_operator_intents
WHERE process_id = 'proc-pc-7046e8ec-2734-ed11-8119-005056b64545';
-- After §3 step 9, dgu-002 must be 'stale' if not yet withdrawn

SELECT '=== R.13 stage tracking ===';
SELECT stage, scope_kind, state, COUNT(*) FROM process_stage_status
WHERE stage = 'operator decision' GROUP BY 1,2,3;
SELECT stage, scope_kind, COUNT(*) FROM process_stage_heads
WHERE stage = 'operator decision' GROUP BY 1,2;

SELECT '=== R.14 FK orphans ===';
SELECT 'op_overrides → process: ' || COUNT(*) FROM operator_input_overrides o
LEFT JOIN procurement_processes p ON p.id = o.process_id WHERE p.id IS NULL;
SELECT 'op_overrides → candidate: ' || COUNT(*) FROM operator_input_overrides o
LEFT JOIN product_candidates pc ON pc.id = o.product_candidate_id WHERE pc.id IS NULL;
SELECT 'op_overrides → superseded_by: ' || COUNT(*) FROM operator_input_overrides o
LEFT JOIN operator_input_overrides s ON s.id = o.superseded_by_override_id
WHERE o.superseded_by_override_id IS NOT NULL AND s.id IS NULL;
SELECT 'pending_intents → process: ' || COUNT(*) FROM pending_operator_intents i
LEFT JOIN procurement_processes p ON p.id = i.process_id WHERE p.id IS NULL;
SELECT 'pending_intents → superseded_by: ' || COUNT(*) FROM pending_operator_intents i
LEFT JOIN pending_operator_intents s ON s.id = i.superseded_by_intent_id
WHERE i.superseded_by_intent_id IS NOT NULL AND s.id IS NULL;

SELECT '=== R.15 Stage 12/13 invariants intact (NON-DGU rows unchanged) ===';
SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY 1;
SELECT pipeline_state, COUNT(*) FROM pipeline_assessments GROUP BY 1;
SELECT stoplight, COUNT(*) FROM manifests GROUP BY 1;
-- Stage 12 row count = 17; pipeline_assessments = 7 manifest_ready; manifests = 7 (probably still all clarify, OR DGU may have flipped to pursue if overrides made it pass).
"
```

R.4 demo flow runs the §3 commands inline — capture all stdout from each `cargo run -p daemon -- --capture-* / --withdraw-* / --list-decisions` invocation into `/tmp/stage14_demo.log` and reference it from the validation manifest.

---

## §7 — FAILURE-MODE PLAYBOOK

| Symptom | Likely cause | Remediation |
|---|---|---|
| `--capture-decision` fails with "no manifests row for process_id" | Operator tried to decide on a process that hasn't been through Stage 13 | Run `--ingest manifest` first. Don't relax this check — deciding without a manifest defeats the audit-trail purpose |
| `disposition='hold'` rejected | 3-state enforcement (D1) | Use `watch` instead. If the operator genuinely needs `hold` semantics later, that's a follow-up bead with a CHECK-rewrite migration |
| Same idempotency_key produces 2 rows | UNIQUE constraint not honored at insert time | The daemon must INSERT with explicit error handling: SQLITE_CONSTRAINT → return existing row. Check `params!` placement and error mapping |
| Override row created but Stage 11 still uses supplier capacity | `read_active_override` not called from `infer_order_quantity_basis`, OR called with wrong reason_code list | Re-read §2.5. The injection must filter by `withdrawn_at IS NULL AND superseded_by_override_id IS NULL` |
| Stale detection marks `superseded` intents stale too | UPDATE WHERE clause too broad | Per §2.7: only `status='queued'` should transition to stale. `superseded` and `withdrawn` are terminal-ish; leave alone |
| Pipeline re-run after override doesn't change DGU's basis_hash | Override is being read but not making it into Stage 11's bundle, OR `infer_order_quantity_basis` is bypassed in the rerun path | Verify the rerun goes through the same `run_landed_cost_live_loop` codepath. Check `LandedCostAssessmentRow` carries the candidate_id needed for override lookup |
| `cargo test` fails with epoch mismatch error | `pdf_t_schema_activation_009_default_schema_epoch_is_5` not renamed | Rename to `_is_6` (assert `== 6`); same gotcha as Stages 12 and 13 |
| Migration 0009 applied to live DB but not in git | Same as Stage 13's "uncommitted dangling files" — commit the migration file AND the sqlite.rs registration AND the schema test in one commit | Use `AGENT_NAME=claude-code git commit ...` per agent-mail pre-commit hook |
| `manifest_basis_hash_when_provided` is NULL on overrides | Daemon not reading the latest manifest before insert | Look up `manifests.manifest_basis_hash` in the same transaction as the override insert; pass it through |
| `synthesize_revenue_from_rfq_override` produces wrong revenue | Math error in unit_price × order_qty; OR currency conversion if RFQ currency != GBP | Phase 1 assumes GBP for RFQ overrides. Document this; non-GBP RFQ overrides are out of scope (would need FX skill integration) |
| Modified anything outside the new scope | Out of scope | Stop. Phase 1 scope is: migration 0009, sqlite.rs registration, CLI subcommands, override-injection in Stages 10/11, pending_operator_intents write path, stale detection in Stage 13, T.1–T.18, opd_t schema activation test. Nothing else |

---

## §8 — CONSTRAINTS

1. **Do not modify Stage 11/12/13 code paths beyond the override-injection points** specified in §2.5. Specifically: `run_landed_cost_live_loop`, `run_net_margin_gate_live_loop`, `run_manifest_generation_live_loop`, `validate_manifest_skill_output` stay structurally unchanged. The override read happens INSIDE existing functions (`infer_order_quantity_basis`, the Stage 10 revenue resolver) without rearchitecting them.
2. **Do NOT touch `apps/desktop/`** — Phase 2 is a separate handoff. If `cargo check --all-targets` flags the desktop crate, that's a coincidence; address inline only if needed for the broad gate to pass.
3. **No new dependencies.** `clap`, `serde_json`, `serde_yaml`, `sha2`, `rusqlite` are all available.
4. **3-state disposition is non-negotiable** — adding `hold` requires a CHECK-rewrite migration (drop-and-recreate the table or use a WITHOUT ROWID workaround) which is out of scope and risks breaking the existing schema. If `hold` matters later, do a focused follow-up bead.
5. **Idempotency is non-negotiable.** Every CLI mutation requires an idempotency_key (decisions) or a deterministic key (overrides keyed by tuple). Re-running the same command must be safe.
6. **Stale detection runs in Stage 13.** Don't add a separate background job. The same transaction that writes the new manifest also transitions stale intents.
7. **No silent test exclusion.** `cargo test -p daemon --bin daemon` is necessary but not sufficient. The per-file integration loop is mandatory per AGENTS.md `### Stage Validation Test Gate`.
8. **Migration is `IF NOT EXISTS` for indexes; the table itself MUST use `IF NOT EXISTS` so re-applying is a no-op.** Test the migration on a copy of state.db before committing.
9. **Override payload validation is per-reason-code** — quantity_basis_* expects `{qty, unit}`; awaiting_rfq_quote expects `{unit_price_gbp_pence, lead_time_days, moq, currency}`; no_pricing_signal expects `{variant, ...}`. Reject malformed payloads with clear error messages.
10. **Reason codes must come from a closed enum** in code (`enum OperatorOverrideReason { QuantityBasisUndetermined, QuantityBasisUnaligned, AwaitingRfqQuote, NoPricingSignal }`). The CLI accepts strings and maps; unknown codes are rejected.

---

## §9 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/stage-14-validation-manifest.md`. Use the template below **verbatim**:

```markdown
# Stage 14 Operator Decision (Phase 1 — CLI) Validation Manifest

> Generated: {ISO timestamp}
> Status: {GREEN | RED}

## Summary
{1-paragraph: did the §3 demo work end-to-end? what hashes captured? did stale detection fire?}

## Build Summary
- Migration: `migrations/0009_operator_input_overrides.sql`
- Migration registered: `crates/daemon/src/db/sqlite.rs` MIGRATIONS array (epoch 6)
- Schema activation test: `opd_t_schema_activation_009_*`
- Renamed/replaced epoch test: `pdf_t_schema_activation_009_default_schema_epoch_is_5` → `_is_6`
- New CLI subcommands: `--capture-decision`, `--capture-clarification`, `--withdraw-clarification`, `--withdraw-decision`, `--list-decisions`
- Override injection: `infer_order_quantity_basis` (Stage 11), Stage 10 revenue resolver, no-pricing-signal synthesis path
- Stale detection: integrated into `run_manifest_generation_live_loop` post-write
- New unit tests: T.1–T.18 (all in `mod tests`)

## End-to-End Demo Output (per §3)
{Paste captured stdout from /tmp/stage14_demo.log; key SQL query results inline}

## R.1–R.16 Acceptance Criteria
| # | Criterion | Result | Detail |

## Quality Gates
- cargo fmt --check: {PASS/FAIL}
- cargo check --all-targets: {PASS/FAIL}
- cargo clippy --all-targets -- -D warnings: {PASS/FAIL}
- cargo test -p daemon --bin daemon: {PASS/FAIL — N tests, includes T.1–T.18 (target: 147 total)}
- per-file integration loop: {PASS/FAIL — 39 files}

## Issues Found & Fixed
{Each: symptom → root cause → fix → verification}

## Code Changes
| File | Change |

## Out-of-Scope (Documented, Not Fixed Here)
- Phase 2 Tauri desktop UI (separate handoff)
- 4-state disposition with `hold` (requires CHECK-rewrite migration)
- Non-GBP RFQ override currency conversion (would need FX skill integration)
- Bid workflow execution (Stage 15)
- Remote `operator_decision_heads` table + CAS sync (L4)
```

### Quality gates (the broader gate, mandatory per AGENTS.md)

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon                       # expect 147 tests
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

If any single integration test target fails, fix the root cause. Do **not** weaken or `#[ignore]` to bypass.

---

## §10 — REMINDERS

- ULTRATHINK and be meticulous.
- The single most important success signal is **the §3 end-to-end demo working correctly**. Capture every step's stdout and SQL output in the validation manifest. If the demo stalls, that's the bug; fix root cause, don't paper over.
- Stage 14 Phase 1 is **CLI only**. Do not touch `apps/desktop/`. Phase 2 will reuse the data plumbing this handoff lays down.
- `pending_operator_intents` schema is FIXED at 3 dispositions by CHECK constraint. Don't try to circumvent it; ship 3-state.
- The override injection in Stage 11 is the most architecturally consequential change — Stage 11's `infer_order_quantity_basis` becomes a small extension, but it's read by every landed-cost rerun. Test T.9 enforces correctness; don't ship without it green.
- Stale detection fires when Stage 13 writes a manifest with a new basis_hash. The transition is silent (no operator notification in Phase 1) — Phase 2's UI will surface it.
- The `idempotency_key` is the operator's tool for "I want to be sure this command doesn't double-fire". Trust it; don't generate keys server-side without operator awareness.
- Once the manifest is GREEN, commit and push (matching the project's git hygiene pattern). All new files must be in the commit, including the migration file (avoid the "Stage 13 dangling files" repeat).
- The next stage (Stage 15 — bid workflow) requires `pending_operator_intents.status='queued'` AND matching basis_hash to fire. Stage 14's audit-chain + stale-detection design is what makes that contract possible.
