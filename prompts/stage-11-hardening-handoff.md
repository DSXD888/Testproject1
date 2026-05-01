# Stage 11 Hardening — Make Real-Data Runs Repeatable

ULTRATHINK and be meticulous.

## CONTEXT

Stage 11 (landed cost) just completed its first end-to-end real-data run on the DGU import scenario. The validation manifest at `docs/pipeline/stage-11-real-data-validation-manifest.md` reports `GREEN`, and all 13 PASS criteria (R.1–R.13) are satisfied for that single row. **However**, an independent audit found four gaps that prevent the success from being repeatable for other candidates and other operators:

1. **FX skill_session_id is hardcoded `null`.** The manifest admits this. Lines `crates/daemon/src/main.rs:4309` and `:5311` set `"simple_fx": null` in the `skill_session_ids` JSON unconditionally. When the FX skill runs on real data its session id is dropped, so we lose the audit trail for one of the four Stage 11 skills.
2. **`quoted_quantity` is treated as order quantity, not supplier capacity.** The DGU's selected quote `sq-b22fcf00b7a545d699c3438c42b9027c-1` has `quoted_quantity = 10000 square meter`, which is the Alibaba listing's tier/capacity, not the buyer's order. Stage 11 sized FCL freight on 10,000 m² (300,000 kg → 12 × 40ft containers, ~£31k freight base). For the sibling quote `sq-b22fcf00b7a545d699c3438c42b9027c-0` the quirk is also visible: 5,000 piece capacity. Any candidate whose supplier listing exposes capacity in `quoted_quantity` will produce wildly overstated freight and economic landed cost.
3. **No unit tests cover the new Stage 11 code paths.** Five new behaviors shipped: `claim_job_by_id` (FIFO bypass, 5 callsites in `main.rs` + 89 new lines in `jobs/{claim,runner}.rs`), `stage11_default_issue_severity` (defaulting), the tightened `extract_agent_message_json` (rejects envelopes without `type=agent_message` or without status fields), `recovered_result_after_timeout`, `recovered_result_after_protocol_error`. None have a corresponding `#[test]`. A regression in any of them only surfaces in another live run.
4. **CDP captcha fallback is undocumented and circumstantial.** The DGU run's warnings include `CDP_CAPTCHA_INTERCEPTION` — the live Alibaba page returned a captcha and the shipment-sizing skill fell back to "indexed Alibaba listing evidence for the same URL" (per the manifest's Issues Found section). That fallback path is not coded into the skill or the daemon — it is an emergent agent behavior. The next operator hitting the same listing URL will not necessarily replicate it.

Your job is to make Stage 11 land as `review` (or `decision_ready`) on a fresh run, against a fresh candidate, **deterministically**. Each of the four issues below has an independent, scoped fix; deliver them in order or in parallel, but every one must end with regression coverage so the next regression is caught by `cargo test`, not by another live run.

```
┌──────────────────────────────────────────────────────────────────┐
│                          THE LOOP                                  │
│                                                                    │
│   For each of issues 1..4:                                         │
│     a. Reproduce the gap (write a failing test or capture a log)  │
│     b. Implement the fix                                          │
│     c. Verify the test passes                                     │
│     d. Run the full suite: cargo fmt --check &&                   │
│        cargo check --all-targets &&                                │
│        cargo clippy --all-targets -- -D warnings &&                │
│        cargo test -p daemon --bin daemon                           │
│     e. Confirm the DGU row in state.db is still review-grade.     │
│                                                                    │
│   When all four issues are GREEN: write a hardening manifest      │
│   at docs/pipeline/stage-11-hardening-manifest.md.                │
└──────────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first — single source of truth.

---

## ISSUE 1 — FX skill_session_id wiring

### Symptom

```sql
sqlite> SELECT json_extract(confidence_breakdown,'$.skill_session_ids.simple_fx')
        FROM landed_cost_estimates
        WHERE supplier_quote_id = 'sq-b22fcf00b7a545d699c3438c42b9027c-1';
NULL
```

But the daemon log line for that run shows `fx_skill_runs=1`. The skill ran, the session and job exist on disk, the row's `fx_status = 'converted'` — only the session_id pointer is missing.

### Root cause

`crates/daemon/src/main.rs`:

- Line 4305–4310 (initial population): `"simple_fx": null`.
- Line 5305–5311 (overwrite path during the live loop): also sets `"simple_fx": null`.
- Lines 4307, 4308 already wire `hmrc_customs`, `shipment_sizing`, `searates_freight`, plus their `*_job_id` keys. The FX path simply forgot to thread its `session_id` and `job_id` through.

### Fix

1. Plumb the FX skill's `session_id` and `job_id` through `run_landed_cost_skill` / `execute_landed_cost_skill_session` / `build_import_landed_cost_estimate` the same way the other three skills do.
2. Set `skill_session_ids.simple_fx = <session_id>` and `simple_fx_job_id = <job_id>` whenever the FX skill actually ran (i.e. when `fx_status != 'not_required'`). Leave them `null` only when FX is genuinely skipped.

### Regression test

Add a unit test in `crates/daemon/src/main.rs` `mod tests`:

- Build a synthetic `LandedCostAssessmentRow` with a non-GBP freight quote.
- Stub `run_landed_cost_skill` to return a successful `SimpleFxLookupResult` and a known session/job id pair (use the existing test patterns for `hmrc_customs_output_parser_accepts_rebar_safeguard_result` as the shape model).
- Call `build_import_landed_cost_estimate` (`main.rs:4331`) and `persist_landed_cost_estimate` (`main.rs:4837`).
- Assert `confidence_breakdown.skill_session_ids.simple_fx` and `confidence_breakdown.simple_fx_job_id` equal the stubbed values.

If `build_import_landed_cost_estimate` does not currently take FX session ids as inputs, that is the bug — extend its signature and the call site at `main.rs:5339`.

---

## ISSUE 2 — Order-quantity inference (`quoted_quantity` is supplier capacity, not order qty)

### Symptom

DGU selected quote: `unit_price_gbp_pence=516`, `quoted_quantity=10000 square meter`. Stage 11's sizing skill correctly took 10,000 m² as the freight basis (`300,000 kg`, 12 × 40ft FCL). For Stage 11's purposes that produced an internally consistent row. But that 10,000 m² is the **Alibaba listing's tier/capacity**, not the buyer's actual order quantity.

The buyer's order quantity should come from one of these (in priority order):
1. The contract's notice/lot value divided by the unit price (when contract value is set and revenue basis is `expected_value`).
2. The lot's quantity field if the qualify-lot stage extracted one.
3. An explicit RFQ quantity if Stage 10 computed one.
4. Only as last resort: the quote's `minimum_order_quantity` (= `10` for DGU, far smaller than 10,000).

Using supplier capacity instead can overstate freight by 100× for service-priced retail SKUs where listings expose tier-based capacity.

### Root cause

`crates/daemon/src/main.rs:4193–4500` (`build_domestic_landed_cost_estimate` and `build_import_landed_cost_estimate`) and the `shipment-sizing` skill prompt at `skills/landed-cost/shipment-sizing/SKILL.md` both consume `quoted_quantity` directly. Neither layer asks "is this an order quantity or a supplier capacity?".

### Fix

1. Add a small Rust helper `infer_order_quantity_basis(assessment: &LandedCostAssessmentRow) -> OrderQuantityBasis` that returns one of:
   - `OrderQuantityBasis::FromContractValue { qty, contract_value, basis_unit }` — when `revenue_basis_type='expected_value'` and `contract_value/unit_price` produces a sensible quantity.
   - `OrderQuantityBasis::FromLotQuantity { qty, basis_unit }` — when the qualify-lot stage stored a quantity for the lot.
   - `OrderQuantityBasis::RfqRequired { fallback_qty: minimum_order_quantity }` — when both above are absent.
   - `OrderQuantityBasis::SupplierCapacity { qty }` — explicit, only if 1/2/3 yield nothing.
2. Pass the inferred basis into the shipment-sizing skill's input contract as a new field `inferred_order_quantity_basis: { kind: ..., qty: ..., unit: ..., source: ... }`. Update `skills/landed-cost/shipment-sizing/SKILL.md` to require the skill to size on this field, falling back to `quoted_quantity` only when `kind == "supplier_capacity"`.
3. Persist the chosen basis in `confidence_breakdown.order_quantity_basis` so future operators can audit it.
4. Add a `screening_only_quantity_uncertainty` warning when the basis is `SupplierCapacity` or `RfqRequired` — these are valid but should be visible to the operator.

### Regression test

- Unit-test `infer_order_quantity_basis` for the four cases above with synthetic assessments.
- Add an integration-style fixture test that builds a DGU-shaped `LandedCostAssessmentRow` with `revenue_basis_type='rfq_required'` and `quoted_quantity=10000`, asserts the basis comes out `RfqRequired { fallback_qty: 10 }`, and verifies the resulting freight is computed against the small fallback rather than 10,000.

### Note on the existing DGU row

The DGU row currently in the DB was sized on supplier capacity. After this fix, **delete and re-run** the DGU landed-cost row (per the snippet in `prompts/stage-11-real-data-validation-handoff.md`). Confirm the new row is materially smaller (likely 1–4 containers, single-digit-thousand pounds freight) and update the validation manifest accordingly.

---

## ISSUE 3 — Regression test coverage for new Stage 11 code paths

### Symptom

`grep -n "claim_job_by_id\|stage11_default_issue_severity\|extract_agent_message_json\|recovered_result" crates/daemon/src/` returns implementations but zero `#[test]` functions covering them. Five new behaviors shipped without unit tests:

| Behavior | File:line | Test status |
|---|---|---|
| `claim_job_by_id` (FIFO bypass for `--ingest`) | `jobs/claim.rs`, `jobs/runner.rs` | none |
| `stage11_default_issue_severity` (defaults missing severity) | `main.rs:3011` | none |
| `extract_agent_message_json` tightening (now requires `type='agent_message'` AND one of the four status fields) | `main.rs:4175–4196` | none |
| `recovered_result_after_timeout` (kept skill output when bound expired but `result.json` was already written) | `main.rs:3987–4001` | none |
| `recovered_result_after_protocol_error` (kept skill output when stdout/stderr parser failed but `result.json` was already written) | `main.rs:4060–4079` | none |

Several of these are subtle and have already been observed papering over real failures (the manifest's run_7 succeeded only because the recovery path activated — without a test, a future refactor that drops the recovery silently regresses Stage 11 to "blocked".)

### Fix

Add `#[test]` (and `#[tokio::test]` where async) functions in the `mod tests` block of the relevant files. For each behavior, write at least one happy-path and one negative test. Concrete test cases:

1. **`claim_job_by_id_returns_specific_job_when_older_jobs_exist`** (`jobs/claim.rs`)
   - Insert a workflow_job with `status='pending'` and an older sibling pending job.
   - Call `claim_job_by_id` with the *newer* job's id.
   - Assert it returns the newer job, not the older one (FIFO is bypassed).
   - Assert the job's lease/status is updated for the calling executor.

2. **`claim_job_by_id_returns_none_for_nonexistent_or_completed_jobs`** (`jobs/claim.rs`)
   - Same setup; ask for a job id that's `succeeded`. Assert `None`.

3. **`stage11_issue_defaults_severity_when_skill_omits_field`** (`main.rs`)
   - Deserialize a `Stage11Issue` from JSON `{"code":"X","message":"M"}` (no severity).
   - Assert `severity == "warning"`.

4. **`stage11_issue_defaults_message_when_skill_omits_field`** (`main.rs`)
   - Deserialize from `{"code":"X","severity":"blocked"}`.
   - Assert `message == ""` and the issue is still usable downstream.

5. **`extract_agent_message_json_rejects_non_agent_message_envelopes`** (`main.rs`)
   - Pass an envelope with `item.type = "tool_use"` but otherwise valid stage-11 status fields. Assert `None`.

6. **`extract_agent_message_json_rejects_envelopes_missing_status_fields`** (`main.rs`)
   - Pass an envelope with `type=agent_message` but the inner JSON has no `classification_status` / `sizing_status` / `freight_status` / `fx_status`. Assert `None`.

7. **`extract_agent_message_json_accepts_well_formed_freight_result`** (`main.rs`)
   - Pass a SeaRates-shaped `agent_message`. Assert it parses to the inner object.

8. **`landed_cost_recovers_result_after_timeout`** (`main.rs`)
   - Drive a fake `execute_landed_cost_skill_session` (or refactor to take an injectable child-process) where the child writes `result.json` then sleeps past the 300s deadline.
   - Assert: the function does NOT bail with "timed out", returns success, and the persisted `confidence_breakdown.recovered_result_after_timeout = true`.
   - If full async harness is too heavy, refactor the recovery decision into a pure function `decide_landed_cost_outcome(timed_out: bool, parser_errors: Option<String>, result_path: Option<PathBuf>) -> LandedCostOutcome` and unit-test that.

9. **`landed_cost_bails_on_timeout_without_result`** (`main.rs`)
   - Same as above but `result_path = None` after timeout. Assert the function bails with the timeout error message.

10. **`landed_cost_recovers_result_after_protocol_parse_error`** (`main.rs`)
    - Same as 8 but the timeout flag is false and the parser error is `Some("stdout: ...")`. Assert success with `recovered_result_after_protocol_error = true`.

The pure-function refactor in 8/10 is preferred — it isolates the policy decision from the I/O scaffolding and makes future regressions trivial to catch.

### Regression test pass criteria

After this issue is done, `grep -c '#\[test\]\|#\[tokio::test\]' crates/daemon/src/main.rs crates/daemon/src/jobs/claim.rs crates/daemon/src/jobs/runner.rs` should increase by at least 8. `cargo test -p daemon --bin daemon` should report **all** tests passing (currently 59/60; the 60th is unrelated — see Issue 4 of the separate `stage-11-pdf-policy-test-fix-handoff.md`).

---

## ISSUE 4 — CDP captcha resilience

### Symptom

The DGU run's `warnings` array contains `CDP_CAPTCHA_INTERCEPTION` — the live Alibaba listing was captcha-blocked. Per the validation manifest's Issues Found:

> Alibaba page access was captcha-gated in CDP. Root cause: Alibaba returned "Captcha Interception" for the exact listing URL. Fix applied: no bypass was attempted; sizing used indexed Alibaba listing evidence for the same URL and documented `CDP_CAPTCHA_INTERCEPTION`.

"Indexed Alibaba listing evidence" is not a coded fallback path — it is an emergent behavior of the agent loop. The next operator running fresh against a fresh listing URL has no guarantee of replicating it.

### Fix scope (this is the smaller of the four issues; do it last or open a follow-up bead)

1. Decide on a deterministic captcha-recovery policy. Options:
   - **(a)** When Alibaba returns captcha, the sizing skill records `cdp_listing_state = "captcha_blocked"` and proceeds to size from cached listing metadata stored in `state.db` (none today — would require a new `marketplace_listing_snapshots` table) or from the search-attempt evidence already in `search_attempts.evidence_json`.
   - **(b)** When captcha hits, mark the row `decision_grade='blocked'` with a `CDP_CAPTCHA_BLOCKED` blocker and re-route to a manual operator step. This is honest but stops the pipeline.
   - **(c)** Add a CDP-side captcha solver / wait-for-human path. Out of scope for L1-L3 MVP.
2. Whichever option you pick, document it in `skills/landed-cost/shipment-sizing/SKILL.md` so the agent applies it consistently.
3. Add a unit test for the chosen policy (e.g. when the sizing skill's input contains `cdp_listing_state='captcha_blocked'`, the daemon either persists the row with the captcha-blocked grade or accepts the cached-fallback path with the right warning).

If the team is not ready to commit to a policy, **at minimum** make the daemon **not** accept a row as `review` if captcha was hit and no deterministic fallback was used. That prevents the false-confidence outcome we just shipped.

---

## DELIVERABLE

### MANDATORY: Hardening manifest

Path: `docs/pipeline/stage-11-hardening-manifest.md`. Same template shape as the existing real-data manifest. For each of the four issues, document:

| Issue | Status (GREEN/YELLOW/RED) | Tests added | Behavior verified | Remaining risk |
|---|---|---|---|---|

Append a "Re-run on the DGU" section: re-execute `cargo run -p daemon -- --ingest landed-cost` (with the existing CDP setup), verify the DGU row is still `review`, and paste the new daemon log line.

### Quality gates

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon                       # ALL tests, no filter
```

The full-suite test must be green. **Do not declare done with any failing tests, even unrelated ones** — if you find one, file a follow-up bead and document it in the manifest's "Out-of-Scope" section.

---

## CONSTRAINTS

1. **Do not weaken existing PASS criteria from the real-data manifest.** R.1–R.13 must still pass after every fix.
2. **Do not regress the 3 domestic landed-cost rows.** They should remain `review` with `economic_landed_cost_gbp_pence == supplier_cost_gbp_pence` and zeros elsewhere.
3. **No skill prompt changes that simulate or fake live results.** Skill prompts may improve their handling of edge cases, but the live HMRC / SeaRates / Frankfurter / CDP calls must remain real.
4. **Each test must run in `cargo test` without external services.** Pure-function refactors and DI shims are acceptable; live HTTP calls in tests are not.

---

## REMINDERS

- ULTRATHINK and be meticulous.
- The Stage 11 real-data manifest is GREEN for one row only. This handoff makes that result repeatable for all future runs.
- All four issues have a clear test deliverable. After this work, a regression in any of these paths fails `cargo test`, not the next live run.
