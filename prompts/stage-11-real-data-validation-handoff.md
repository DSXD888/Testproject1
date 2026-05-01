# Stage 11 Real-Data Validation (DGU End-to-End)

ULTRATHINK and be meticulous.

## MISSION

Stage 11 (Landed Cost) shipped as MVP with all four skills (HMRC customs, shipment sizing, SeaRates freight, simple FX) wired in, but **no live import row has yet completed end-to-end**. The only successful real-data rows so far are 3 trivial domestic passthroughs (`amazon-uk` GB, all zeros). The one real import candidate — the **Double-Glazed Unit (DGU)** from `alibaba-search` — currently sits as `decision_grade=blocked` because the persistent CDP browser was down on the previous run.

Your job: bring up CDP, drive the DGU through `--ingest landed-cost`, and produce the **first real review-grade import landed-cost row in this database**. Then write a manifest documenting whether the full skill chain works on real data.

If the DGU row stays `blocked`, this validation **fails** — investigate, fix the blocker, and re-run until the row lands as `review` (or `decision_ready` if confidence is high enough).

```
┌──────────────────────────────────────────────────────────────────┐
│                          THE LOOP                                  │
│                                                                    │
│   1. Pre-flight: CDP up, DB snapshot captured                     │
│   2. Trigger: cargo run -p daemon -- --ingest landed-cost         │
│   3. Inspect: did the DGU row transition blocked → review?        │
│   4. Any failure? (skill timeout, CDP error, parse error, ...)    │
│      ├─ YES → Diagnose root cause, fix, clear the DGU row,        │
│      │        re-run from step 2                                  │
│      └─ NO  → Validate per the PASS criteria                      │
│   5. All checks green? Write manifest. Otherwise back to step 4.  │
│                                                                    │
│   DO NOT STOP UNTIL THE DGU ROW IS REVIEW-GRADE.                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## REAL DATA INVENTORY

### What we have

| Field | Value |
|---|---|
| Process | `proc-pc-7046e8ec-2734-ed11-8119-005056b64545` |
| Notice  | `pc-7046e8ec-2734-ed11-8119-005056b64545` |
| Product candidate | `pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279` |
| Candidate name | `Double-glazed unit (sealed insulating glass unit, BS EN 1279-compliant, argon-filled, warm-edge spacer)` |
| Selected quote | `sq-b22fcf00b7a545d699c3438c42b9027c-1` |
| Quote source | `alibaba-search`, `ship_from_country = CN`, `currency = GBP` |
| Quote URL | `https://www.alibaba.com/product-detail/Low-E-Tempered-Insulated-Glass-Clear_1600798460524.html` |
| `unit_price_gbp_pence` | `516` (already normalized to GBP at Stage 9) |
| `quoted_quantity` | `10000` `square meter` (supplier capacity — likely not order qty; flag if it distorts freight) |
| `minimum_order_quantity` | `10` |
| `row_version` | `1` |
| Stage 10 pricing assessment | `pricing-pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279` |
| Current landed-cost row id | `landed-pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279-sq-b22fcf00b7a545d699c3438c42b9027c-1` |
| Current decision grade | `blocked` |
| Current blockers | `CUSTOMS_SKILL_NO_RESULT`, `CDP_CONNECT_FAILED`, `SEARATES_NO_RESULT` |
| Current supplier_cost / VAT / economic | 5,160,000p / 1,032,000p / 6,192,000p |

### What we don't have (out of scope for this validation)

- **No non-GBP quotes** anywhere in the DB (`SELECT COUNT(*) FROM supplier_quotes WHERE currency != 'GBP'` = 0). The simple-fx-lookup skill cannot be exercised on real data — only its parser test (`searates_output_parser_accepts_quoted_result`'s sibling) covers it. Do **not** seed fake currencies; flag this as "FX path not exercised" in the manifest and move on.
- **Only one HS family** (insulating glass units → likely HS chapter 7008) and one origin country (CN). Multi-origin / multi-family validation is future work.
- The other 3 quoted candidates are all `amazon-uk` GB and trivially passthrough — re-running on them does not exercise customs/freight/FX.

### Likely HS code

DGU corresponds to "multiple-walled insulating units of glass". The 10-digit family in the UK tariff is **`7008.00.20.00`** (multiple-walled insulating units of glass, sealed). The HMRC customs skill should return this or a sibling 10-digit code. If it returns something outside chapter 7008, flag it — but do not override.

---

## PRE-FLIGHT: CDP Browser Setup

The Stage 11 skills (`hmrc-customs-classification`, `searates-freight-estimate`, `shipment-sizing`) all require a persistent Chromium with CDP exposed on `localhost:9222`. **Before running** the daemon:

```bash
# Verify CDP is reachable
curl -sf http://localhost:9222/json/version || echo "CDP DOWN"
```

If CDP is down, bring it up the way this project normally does — **do not invent a fresh launch command**. Search the repo first:

```bash
grep -rn "9222\|--remote-debugging-port\|launchPersistentContext" \
  --include='*.md' --include='*.ts' --include='*.rs' --include='*.json' \
  | head -40
```

Look for an existing skill / runtime helper that opens the persistent profile. Reuse it. If you cannot find one, ask the user before launching Chromium ad-hoc — the user has a specific persistent profile this project depends on for Alibaba session state.

Once CDP is up, smoke-test it can reach the target hosts:

```bash
curl -sf http://localhost:9222/json/version | head -5
# (later) verify the SeaRates skill can hit www.searates.com via CDP from a sandboxed Node script if available
```

**Do not proceed to step 2 of the loop until CDP is reachable.** A Stage 11 run with CDP down will reproduce the existing `CDP_CONNECT_FAILED` blocker — that is not a useful test.

---

## STEP 2: TRIGGER THE STAGE 11 LIVE LOOP

```bash
# From repo root
cargo run -p daemon -- --ingest landed-cost
```

The dispatch lives at `crates/daemon/src/main.rs:1159`. It calls `run_landed_cost_live_loop` (`main.rs:5038`), which:

1. Loads Stage 10 assessments needing Stage 11 (`load_landed_cost_assessments`, `main.rs:3600`) — drift detection looks at `selected_quote_id`, `selected_quote_row_version`, and `supplier_quote.row_version`.
2. For each assessment with `ship_from_country != 'GB'`, runs the four skills in order, bounded by `LANDED_COST_SKILL_TIMEOUT_SECONDS = 90` (`main.rs:251`).
3. Persists a `landed_cost_estimates` row via `persist_landed_cost_estimate` (`main.rs:4837`).

**Expected log line on success:**

```
landed-cost live loop completed: attempted=N completed=M blocked=K domestic=D import=I customs_skill_runs=A sizing_skill_runs=B searates_skill_runs=C fx_skill_runs=F
```

For this validation, you should see at minimum `import=1`, `customs_skill_runs=1`, `sizing_skill_runs>=1`, `searates_skill_runs>=1`. If any of those is 0, a skill bailed early — investigate.

### Forcing the DGU row to re-run

The persisted DGU row (`decision_grade=blocked`) will **not** be re-attempted unless drift is detected. If you've fixed CDP and want to re-run cleanly without changing quote data, delete the row and its stage tracking first:

```bash
sqlite3 state/state.db "
DELETE FROM landed_cost_estimates
  WHERE id = 'landed-pc-7046e8ec-2734-ed11-8119-005056b64545-lot-1-candidate-glass-dgu-bs-en-1279-sq-b22fcf00b7a545d699c3438c42b9027c-1';
DELETE FROM process_stage_heads
  WHERE stage = 'landed cost' AND scope_kind = 'candidate_quote'
    AND scope_key LIKE '%glass-dgu%';
DELETE FROM process_stage_status
  WHERE stage = 'landed cost' AND scope_kind = 'candidate_quote'
    AND scope_key LIKE '%glass-dgu%';
"
```

Snapshot before this delete (for the manifest):

```bash
sqlite3 state/state.db ".dump landed_cost_estimates" > /tmp/landed_cost_before.sql
```

---

## STEP 3 / 4: PASS CRITERIA

The DGU row must satisfy **all** of the following after the run. If any fails, loop back to step 2.

| # | Check | PASS condition |
|---|---|---|
| R.1 | Row exists, `decision_grade != 'blocked'` | `decision_grade IN ('review', 'decision_ready')` |
| R.2 | Real HS-10 commodity code | `declarable_commodity_code_10` is non-NULL, length 10, starts with `7008` for DGU (or the agent has documented why a different chapter is correct) |
| R.3 | Customs skill ran successfully | `customs_resolution_confidence_bps > 0`; `commodity_confidence_bps >= 5000`; no `CUSTOMS_SKILL_NO_RESULT` in `blockers` |
| R.4 | Freight skill returned a quote | `freight_base_gbp_pence > 0` AND `freight_low <= freight_base <= freight_high`; no `SEARATES_NO_RESULT` in `blockers` |
| R.5 | Shipment sizing populated | `packing_scenario` is non-NULL |
| R.6 | Duty regime is real | `customs_duty_gbp_pence` matches a documented MFN / preferential / safeguard outcome (could be 0 if duty-free under MFN, but `duty_breakdown_json` must show the source) |
| R.7 | VAT is from a real measure | `vat_rate_state` is set (e.g. `standard` / `zero` / `reduced`); `import_vat_treatment` reflects MVP policy |
| R.8 | Economic landed cost is consistent | `economic_landed_cost_gbp_pence == supplier_cost + freight_base + customs_duty + trade_remedy_duty + import_vat + uk_final_mile + other_fees` (within rounding) |
| R.9 | Net margin band is set | `net_margin_low_bps <= net_margin_base_bps <= net_margin_high_bps`; values numerically valid (DGU revenue is `rfq_required` so margins may be 0 — that's acceptable, but the band must populate) |
| R.10 | Stage tracking advanced | One `process_stage_heads` row at `('landed cost','candidate_quote', dgu_scope)` with state `succeeded`; no orphans |
| R.11 | No FK orphans | `landed_cost_estimates` joins cleanly to `procurement_processes`, `contract_notices`, `product_candidates`, `supplier_quotes` |
| R.12 | Quote-version freshness | `confidence_breakdown.selected_quote_row_version` and `.supplier_quote_row_version` match the live `cpa.selected_quote_row_version` and `sq.row_version` |
| R.13 | All other rows still healthy | The 3 domestic rows remain `review`, all FK orphans = 0, total `landed_cost_estimates` count = 4 (or higher if reruns produced extras for the same scope — investigate if so) |

---

## VALIDATION QUERIES

Run these after each Stage 11 attempt:

```bash
sqlite3 -header -column state/state.db "
SELECT '=== R.1-R.7 DGU row ===';
SELECT decision_grade AS grade,
       declarable_commodity_code_10 AS hs10,
       commodity_confidence_bps AS commodity_conf,
       customs_resolution_confidence_bps AS customs_conf,
       packing_scenario AS pack,
       trade_remedy_state AS remedy_state,
       vat_rate_state AS vat_state,
       import_vat_treatment AS vat_treatment
FROM landed_cost_estimates
WHERE supplier_quote_id = 'sq-b22fcf00b7a545d699c3438c42b9027c-1';

SELECT '=== R.4-R.6 freight/duty ===';
SELECT freight_low_gbp_pence AS f_low, freight_base_gbp_pence AS f_base, freight_high_gbp_pence AS f_high,
       customs_duty_gbp_pence AS duty, trade_remedy_duty_gbp_pence AS remedy, import_vat_gbp_pence AS vat,
       uk_final_mile_cost_gbp_pence AS final_mile, other_fees_gbp_pence AS other,
       supplier_cost_gbp_pence AS supplier, economic_landed_cost_gbp_pence AS economic
FROM landed_cost_estimates
WHERE supplier_quote_id = 'sq-b22fcf00b7a545d699c3438c42b9027c-1';

SELECT '=== R.8 arithmetic check ===';
SELECT (supplier_cost_gbp_pence + freight_base_gbp_pence + customs_duty_gbp_pence
        + COALESCE(trade_remedy_duty_gbp_pence,0) + import_vat_gbp_pence
        + uk_final_mile_cost_gbp_pence + other_fees_gbp_pence) AS computed,
       economic_landed_cost_gbp_pence AS stored,
       (economic_landed_cost_gbp_pence -
        (supplier_cost_gbp_pence + freight_base_gbp_pence + customs_duty_gbp_pence
         + COALESCE(trade_remedy_duty_gbp_pence,0) + import_vat_gbp_pence
         + uk_final_mile_cost_gbp_pence + other_fees_gbp_pence)) AS delta
FROM landed_cost_estimates
WHERE supplier_quote_id = 'sq-b22fcf00b7a545d699c3438c42b9027c-1';

SELECT '=== R.9 margin band ===';
SELECT net_margin_low_bps AS low, net_margin_base_bps AS base, net_margin_high_bps AS high
FROM landed_cost_estimates
WHERE supplier_quote_id = 'sq-b22fcf00b7a545d699c3438c42b9027c-1';

SELECT '=== R.10 stage tracking ===';
SELECT stage, scope_kind, state, COUNT(*) AS n
FROM process_stage_status WHERE stage = 'landed cost'
GROUP BY stage, scope_kind, state;
SELECT stage, scope_kind, COUNT(*) AS n
FROM process_stage_heads WHERE stage = 'landed cost'
GROUP BY stage, scope_kind;

SELECT '=== R.11 FK orphans ===';
SELECT 'orphan process: ' || COUNT(*) FROM landed_cost_estimates lce
  LEFT JOIN procurement_processes p ON p.id = lce.process_id WHERE p.id IS NULL;
SELECT 'orphan notice: ' || COUNT(*) FROM landed_cost_estimates lce
  LEFT JOIN contract_notices n ON n.id = lce.notice_id WHERE n.id IS NULL;
SELECT 'orphan candidate: ' || COUNT(*) FROM landed_cost_estimates lce
  LEFT JOIN product_candidates pc ON pc.id = lce.product_candidate_id WHERE pc.id IS NULL;
SELECT 'orphan quote: ' || COUNT(*) FROM landed_cost_estimates lce
  LEFT JOIN supplier_quotes sq ON sq.id = lce.supplier_quote_id WHERE sq.id IS NULL;

SELECT '=== R.12 quote-version freshness ===';
SELECT json_extract(confidence_breakdown,'\$.selected_quote_row_version') AS sel_qrv,
       json_extract(confidence_breakdown,'\$.supplier_quote_row_version') AS sq_qrv,
       cpa.selected_quote_row_version AS cpa_sel,
       sq.row_version AS sq_live
FROM landed_cost_estimates lce
JOIN candidate_pricing_assessments cpa
  ON cpa.product_candidate_id = lce.product_candidate_id
 AND cpa.selected_quote_id   = lce.supplier_quote_id
JOIN supplier_quotes sq ON sq.id = lce.supplier_quote_id
WHERE lce.supplier_quote_id = 'sq-b22fcf00b7a545d699c3438c42b9027c-1';

SELECT '=== R.13 cohort health ===';
SELECT decision_grade, COUNT(*) FROM landed_cost_estimates GROUP BY decision_grade;
SELECT 'total: ' || COUNT(*) FROM landed_cost_estimates;
"
```

Also dump the full DGU row for the manifest:

```bash
sqlite3 state/state.db "
SELECT * FROM landed_cost_estimates
WHERE supplier_quote_id = 'sq-b22fcf00b7a545d699c3438c42b9027c-1';
" > /tmp/dgu_landed_cost_after.txt
```

---

## FAILURE-MODE PLAYBOOK

If a check fails, diagnose root cause first — **do not** paper over with synthetic data, do **not** skip the row, do **not** weaken PASS criteria.

| Symptom | Likely cause | Remediation |
|---|---|---|
| `CDP_CONNECT_FAILED` re-appears | CDP died between pre-flight and skill exec | Re-launch persistent browser, verify with `curl -sf localhost:9222/json/version`, retry |
| `CUSTOMS_SKILL_NO_RESULT` after 90s | HMRC API slow / agent prompt loop too long | Look at `crates/daemon/src/main.rs:251` (timeout const). Check the SKILL.md instructions in `skills/landed-cost/hmrc-customs-classification/`. If 90s is too tight for a real agent loop, document this and either tune the constant (small code change) or simplify the SKILL.md prompt — do **not** simulate the result |
| HMRC returns wrong chapter (not 7008) | Skill prompt drift, or the candidate description led the model astray | Re-read `skills/landed-cost/hmrc-customs-classification/SKILL.md`. Fix the prompt or schema. Re-run |
| `SEARATES_NO_RESULT` even with CDP up | SeaRates page changed, login/captcha gate, sizing input missing | Inspect via the browser. If sizing is the blocker, fix `shipment-sizing` first. If SeaRates UI changed, update its SKILL.md selectors / fallback strategy. Document the change |
| Economic landed cost arithmetic off | `build_import_landed_cost_estimate` bug (`main.rs:4331`) | Add a regression test alongside `landed_cost_arithmetic_populates_net_margin_band` (`main.rs:15604`) |
| Quote-version drift detected mid-run | Stage 9 wrote a new quote concurrently | Snapshot, retry — and document. Should not happen in this validation since no Stage 9 is running |
| The supplier_cost ballooned via `quoted_quantity = 10000` | The candidate cost basis multiplied unit price by capacity, not order qty | This is a known quirk worth flagging in the manifest. Do **not** patch in this validation — log it as Issues Found and propose a fix bead |

---

## CONSTRAINTS

1. **No fake data.** Do not seed currencies, dimensions, freight quotes, or HS codes. If a skill cannot return a real result, the row stays blocked and you fix the skill — not the data.
2. **No bypassing blockers.** Do not edit `landed_cost_estimates.decision_grade` directly. The grade comes from `persist_landed_cost_estimate`'s logic. Fix the inputs, not the verdict.
3. **No weakening PASS criteria.** R.1–R.13 are the contract. If a check is genuinely wrong (e.g., DGU is genuinely duty-free under MFN and 0 is the right answer), **document the reasoning in the manifest** with the HMRC measure evidence, and treat that PASS criterion as met.
4. **Loop until green.** Do not declare "done" with `decision_grade=blocked`. If after several attempts CDP cannot be made to work in this environment, that is itself a finding — write a manifest with status `RED` documenting the blocker rather than calling it green.
5. **Rust hygiene if any code changes:** `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test -p daemon --bin daemon landed_cost`.
6. **Read AGENTS.md** before starting — single source of truth for agent instructions.

---

## DELIVERABLE

### MANDATORY: Write the validation manifest

Path: `docs/pipeline/stage-11-real-data-validation-manifest.md`

Template:

```markdown
# Stage 11 Real-Data Validation Manifest (DGU End-to-End)

> Generated: {ISO timestamp}
> Status: {GREEN | YELLOW | RED}

## Summary

{1-paragraph plain-English summary: did the DGU import row complete end-to-end on real
HMRC + SeaRates + sizing data? What HS code did we get? What freight did we get? What
duty? What was the final economic landed cost and net margin band?}

## Pre-Flight

| Item | Status |
|---|---|
| CDP `localhost:9222/json/version` | OK / FAILED |
| Persistent browser launch command | {paste the command actually used} |
| DGU row before run | blocked (or whatever) |
| Snapshot path | /tmp/landed_cost_before.sql |

## Run Result

```
{paste the "landed-cost live loop completed: ..." log line}
```

## DGU Row After Run

| Field | Value |
|---|---|
| decision_grade | review / decision_ready |
| declarable_commodity_code_10 | 7008.... |
| commodity_confidence_bps | X |
| customs_resolution_confidence_bps | X |
| packing_scenario | {LCL / FCL / etc} |
| supplier_cost_gbp_pence | X |
| freight_low / base / high | X / X / X |
| customs_duty_gbp_pence | X |
| trade_remedy_duty_gbp_pence | X (or NULL) |
| import_vat_gbp_pence | X |
| uk_final_mile_cost_gbp_pence | X |
| other_fees_gbp_pence | X |
| economic_landed_cost_gbp_pence | X |
| net_margin_low / base / high (bps) | X / X / X |
| blockers | [] |
| warnings | [...] |

## PASS Criteria Results

| # | Check | Result | Detail |
|---|---|---|---|
| R.1 | DGU not blocked | PASS / FAIL | {grade} |
| R.2 | Real HS-10 | PASS / FAIL | {code} |
| R.3 | Customs skill ran | PASS / FAIL | {confidence} |
| R.4 | Freight quote | PASS / FAIL | {low/base/high} |
| R.5 | Shipment sizing | PASS / FAIL | {scenario} |
| R.6 | Duty regime | PASS / FAIL | {evidence} |
| R.7 | VAT measure | PASS / FAIL | {state} |
| R.8 | Arithmetic consistent | PASS / FAIL | {delta} |
| R.9 | Margin band populated | PASS / FAIL | {l/b/h} |
| R.10 | Stage tracking | PASS / FAIL | {counts} |
| R.11 | FK integrity | PASS / FAIL | {orphans} |
| R.12 | Quote-version fresh | PASS / FAIL | {versions} |
| R.13 | Cohort still healthy | PASS / FAIL | {grade distribution} |

## Skills Exercised

| Skill | Ran? | Real result? | Notes |
|---|---|---|---|
| hmrc-customs-classification | YES/NO | YES/NO | {hs code, measure types observed} |
| shipment-sizing             | YES/NO | YES/NO | {scenario, dimensions} |
| searates-freight-estimate   | YES/NO | YES/NO | {route, quote validity} |
| simple-fx-lookup            | NO     | NO     | All quotes GBP — FX path not exercised on real data |

## Issues Found

{Each issue: symptom → root cause → fix applied (or fix bead opened)}

## Out-of-Scope (Documented, Not Fixed Here)

- FX path (no non-GBP quotes in DB)
- Multi-origin / multi-HS-family validation
- DGU `quoted_quantity = 10000` quirk (supplier capacity vs order qty) — propose bead if it materially distorts freight
- Anything in `Remaining Limitations` from the original Stage 11 manifest

## Quality Gates

```
cargo fmt --check                          {PASS/FAIL}
cargo check --all-targets                  {PASS/FAIL}
cargo clippy --all-targets -- -D warnings  {PASS/FAIL}
cargo test -p daemon --bin daemon landed_cost  {PASS/FAIL — N tests}
```

## Data Snapshots

- Before: `/tmp/landed_cost_before.sql`
- After:  `/tmp/dgu_landed_cost_after.txt`
- Daemon log: `{path}`
- Validation query output: paste full output of the SELECT block above
```

### If you make any code changes (skill prompts, schema, parsers, timeout)

Add a `Code Changes` section to the manifest listing each file touched and the one-line reason. Run the full quality-gate sweep before declaring GREEN.

---

## REMINDERS

- ULTRATHINK and be meticulous.
- This is the **first real-data import run** — the value of this validation comes from telling us whether the skill chain works on a real Alibaba listing + real HMRC API + real SeaRates UI, not from declaring success.
- A **YELLOW or RED manifest with a clear root-cause writeup is more valuable than a GREEN manifest that papered over a real failure**.
- The DGU's selected quote (`sq-b22fcf00b7a545d699c3438c42b9027c-1`) is the canonical test case. Re-running won't break Stage 9 or Stage 10 — only Stage 11 row state changes.
- If you find a quirk that warrants follow-up but isn't in scope (e.g. the `quoted_quantity = 10000` issue), document it under `Out-of-Scope` and propose a bead — don't expand scope mid-validation.
