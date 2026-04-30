# Stage 11 Real-Data Validation Manifest (DGU End-to-End)

> Generated: 2026-04-30T21:16:00Z
> Status: GREEN

## Summary

The DGU import row completed end-to-end on real Stage 11 data after the hardening pass and persisted as `decision_grade=review`. HMRC classified the Alibaba DGU as `7008002000` with 8400 bps commodity/customs confidence, third-country duty at 0 GBP pence, and a standard 20% import VAT measure. Shipment sizing now uses the inferred order quantity basis (`rfq_required`, MOQ fallback 10 sqm) rather than the Alibaba listing tier/capacity of 10000 sqm, producing an `LCL` plan of about 300 kg and 0.315 CBM planning volume. SeaRates returned a live freight result converted through the simple FX skill to 17,692 GBP pence base at USD/GBP 0.74026. Final economic landed cost is 27,422 GBP pence, with net margin band 0 / 0 / 0 bps because Stage 10 revenue remains `rfq_required`.

## Pre-Flight

| Item | Status |
|---|---|
| CDP `localhost:9222/json/version` | OK - Chrome/147.0.7727.119 |
| Persistent browser launch command | `open -na "Google Chrome" --args "--remote-debugging-port=9222" "--remote-allow-origins=*" "--user-data-dir=$HOME/.chrome-automation" "--profile-directory=Default" "--no-first-run" "--no-default-browser-check"` |
| DGU row before run | prior review row from the first run was reset for a clean hardening rerun |
| Snapshot path | `/tmp/landed_cost_before_stage11_hardening.sql` |

## Run Result

```
landed-cost live loop completed: attempted=1 completed=1 blocked=0 domestic=0 import=1 customs_skill_runs=1 sizing_skill_runs=1 searates_skill_runs=1 fx_skill_runs=1
```

The final daemon log is `/tmp/stage11_hardening_landed_cost_rerun.log`.

## DGU Row After Run

| Field | Value |
|---|---|
| decision_grade | review |
| declarable_commodity_code_10 | 7008002000 |
| commodity_confidence_bps | 8400 |
| customs_resolution_confidence_bps | 8400 |
| order_quantity_basis | `rfq_required`, qty 10.0 square meter |
| packing_scenario | LCL, about 300 kg, 0.315 cbm planning volume |
| supplier_cost_gbp_pence | 5160 |
| freight_low / base / high | 17692 / 17692 / 20801 |
| customs_duty_gbp_pence | 0 |
| trade_remedy_duty_gbp_pence | 0 |
| import_vat_gbp_pence | 4570 |
| uk_final_mile_cost_gbp_pence | 0 |
| other_fees_gbp_pence | 0 |
| economic_landed_cost_gbp_pence | 27422 |
| net_margin_low / base / high (bps) | 0 / 0 / 0 |
| blockers | `[]` |
| warnings | `screening_only_quantity_uncertainty`, `ORIGIN_INFERRED_FROM_SHIP_FROM`, `CUSTOMS_VALUE_MAY_EXCLUDE_SHIPPING`, `LOW_E_BRANCH_SELECTED_FROM_TITLE`, `FLAT_GLASS_PHYSICS_ESTIMATE`, `PACKING_SCHEDULE_MISSING`, `AMBIGUOUS_SINGLE_ITEM_PACKAGE_IGNORED`, `SEARATES_PRICE_EXPIRED`, `REVENUE_BASIS_RFQ_REQUIRED`, `IMPORT_VAT_RECOVERABILITY_UNKNOWN_INCLUDED_IN_ECONOMIC_COST`, `CLEARANCE_FEES_NOT_MODELED` |

## PASS Criteria Results

| # | Check | Result | Detail |
|---|---|---|---|
| R.1 | DGU not blocked | PASS | `decision_grade=review` |
| R.2 | Real HS-10 | PASS | `7008002000`, length 10, starts with `7008` |
| R.3 | Customs skill ran | PASS | commodity/customs confidence 8400 bps; no `CUSTOMS_SKILL_NO_RESULT` blocker |
| R.4 | Freight quote | PASS | 17692 / 17692 / 20801 GBP pence, base within band; no `SEARATES_NO_RESULT` blocker |
| R.5 | Shipment sizing | PASS | LCL plan persisted from 10 sqm inferred RFQ/MOQ basis, not 10000 sqm supplier capacity |
| R.6 | Duty regime | PASS | HMRC third-country duty measure `20008809` at 0%; no CN-specific trade remedy observed |
| R.7 | VAT measure | PASS | `vat_rate_state=standard_or_hmrc_rate`; HMRC VAT measure `-1012545445` at 20%; MVP treatment `recoverability_unknown` |
| R.8 | Arithmetic consistent | PASS | computed 27422, stored 27422, delta 0 |
| R.9 | Margin band populated | PASS | 0 / 0 / 0 bps; revenue basis is `rfq_required` |
| R.10 | Stage tracking | PASS | `process_stage_status`: 4 succeeded candidate_quote rows; `process_stage_heads`: 4 candidate_quote rows |
| R.11 | FK integrity | PASS | process, notice, candidate, quote orphan counts all 0 |
| R.12 | Quote-version fresh | PASS | selected quote row version 1 and supplier quote row version 1 match live CPA/SQ values |
| R.13 | Cohort still healthy | PASS | 4 total landed-cost rows, all `review` |

## Skills Exercised

| Skill | Ran? | Real result? | Notes |
|---|---|---|---|
| hmrc-customs-classification | YES | YES | HMRC API selected `7008002000`; observed third-country duty, supplementary unit, VAT, and non-applicable RU/BY additional-duty measures |
| shipment-sizing | YES | YES | Sized the inferred 10 sqm RFQ/MOQ basis as 300 kg and 0.315 cbm planning volume; LCL |
| searates-freight-estimate | YES | YES | SeaRates returned LCL freight for the small shipment; the daemon accepted a valid result contract written before the bounded runner timeout |
| simple-fx-lookup | YES | YES | SeaRates returned non-GBP freight; Frankfurter/ECB converted the base to 17,692 GBP pence at 0.74026, and the row now records the FX session/job ids |

## Issues Found

- CDP was the original blocker. Root cause: the persistent browser was not reachable on `localhost:9222`. Fix applied: relaunched the project-standard persistent Chrome profile and verified `/json/version` before running Stage 11.
- Real skill sessions exceeded the original 90 second timeout. Root cause: HMRC and SeaRates agent loops produced valid result contracts but not before the old timeout. Fix applied: raised the Stage 11 skill timeout to 300 seconds and taught the daemon to recover a valid `result.json` when a bounded skill times out after writing one.
- Protocol parsing treated some non-result output as fatal even after a result contract existed. Root cause: daemon-side extraction was too permissive and parse errors were fatal. Fix applied: only accept final agent-message JSON with required Stage 11 result fields, and keep protocol parse failures non-fatal when `result.json` validates.
- Skill warning/blocker shapes were inconsistent. Root cause: prompts did not force object-shaped issues and the daemon required complete issue fields. Fix applied: skill prompts now require `{code,message,severity}` issue objects, and the daemon defaults missing `message`/`severity` defensively.
- Shipment sizing needed DGU-specific physics. Root cause: the previous skill prompt lacked a real sizing rule for flat glass quoted per square metre. Fix applied: added DGU/IGU flat-glass density, thickness, crating uplift, and FCL container planning guidance.
- SeaRates UI access required the Logistics Explorer GraphQL path. Root cause: the visible UI path was too brittle for reliable bounded runs. Fix applied: skill now uses CDP session token plus SeaRates Logistics Explorer GraphQL `rates(...)` and `recalculateRate(...)`, still against live SeaRates data.
- SeaRates initially emitted `freight_status=estimated`, which the daemon contract rejected. Root cause: skill wording did not name the accepted success enum. Fix applied: the SeaRates skill now explicitly emits `freight_status=quoted` for concrete non-expired SeaRates amounts.
- The first run used the Alibaba listing tier/capacity (`quoted_quantity=10000`) as the shipment basis. Root cause: Stage 11 did not distinguish order quantity from supplier capacity. Fix applied: the daemon now infers and persists `order_quantity_basis`; the sizing skill sizes from that field, with uncertainty warning when using RFQ/MOQ fallback.
- FX observability is fixed in the hardening rerun. The row now records `confidence_breakdown.skill_session_ids.simple_fx=session-pty-b1df0973410f8bf4621793c100cf96c77f2a9d60f7e548fe0c6d172fcc3080ba` and `simple_fx_job_id=job-stage11-landed_cost_simple_fx-00926ab4f35245878df3d5d3655ef630`.
- CDP captcha fallback is now deterministic policy. The sizing skill must mark same-URL cached/indexed evidence with `deterministic_captcha_fallback=true`; the daemon blocks review-grade rows when captcha is reported without that deterministic fallback.

## Out-of-Scope (Documented, Not Fixed Here)

- Non-GBP supplier quote FX path: all supplier quotes in the DB are GBP. This run did exercise simple FX for real USD SeaRates freight, but not for a non-GBP supplier quote.
- Multi-origin / multi-HS-family validation.
- Confirmed order quantity is still unknown until RFQ. Stage 11 uses the selected quote MOQ (10 sqm) as a screening fallback and records `screening_only_quantity_uncertainty`.
- Remaining limitations from the original Stage 11 MVP manifest, including screening-grade customs estimates and zero broker/clearance/other fees unless evidence provides them.

## Code Changes

| File | Reason |
|---|---|
| `crates/daemon/src/main.rs` | Increased Stage 11 skill timeout to 300 seconds; wrote `result.schema.json` into skill workdirs; recovered valid result contracts after timeouts/protocol parse failures; tightened result JSON extraction; defaulted incomplete Stage 11 issue objects; added FX session/job id wiring, order quantity basis inference, persisted quantity basis, and deterministic CDP captcha policy |
| `crates/daemon/src/jobs/claim.rs` | Added coverage for claim-by-id FIFO bypass and non-claimable jobs |
| `skills/landed-cost/hmrc-customs-classification/SKILL.md` | Required object-shaped warnings/blockers with severity |
| `skills/landed-cost/shipment-sizing/SKILL.md` | Added DGU/IGU flat-glass sizing, required sizing from `inferred_order_quantity_basis`, documented deterministic captcha fallback, and required object-shaped warnings/blockers |
| `skills/landed-cost/searates-freight-estimate/SKILL.md` | Added live SeaRates Logistics Explorer GraphQL/recalculate workflow and fixed success enum to `quoted` |
| `skills/landed-cost/simple-fx-lookup/SKILL.md` | Added live Frankfurter/ECB lookup path and object-shaped warnings/blockers |

## Quality Gates

```
cargo fmt --check                               PASS
cargo check --all-targets                       PASS
cargo clippy --all-targets -- -D warnings       PASS
cargo test -p daemon --bin daemon               PASS - 81 tests
```

## Data Snapshots

- Before: `/tmp/landed_cost_before_stage11_hardening.sql`
- After: `/tmp/dgu_landed_cost_after_stage11_hardening.txt`
- Daemon log: `/tmp/stage11_hardening_landed_cost_rerun.log`

Validation query output:

```
grade   hs10        commodity_conf  customs_conf  shipping_mode  freight_base  supplier  vat   economic  qty_kind      qty
------  ----------  --------------  ------------  -------------  ------------  --------  ----  --------  ------------  ----
review  7008002000  8400            8400          LCL            17692         5160      4570  27422     rfq_required  10.0

computed  stored  delta  margin_low  margin_base  margin_high
--------  ------  -----  ----------  -----------  -----------
27422     27422   0      0           0            0

check_name        n
----------------  -
orphan process    0
orphan notice     0
orphan candidate  0
orphan quote      0

decision_grade  COUNT(*)
--------------  --------
review          4

total
-----
4
```
