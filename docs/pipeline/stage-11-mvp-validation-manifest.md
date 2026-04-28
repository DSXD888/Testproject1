# Stage 11 MVP Landed Cost Validation Manifest

## Status

- Timestamp: 2026-04-28T19:06:24Z
- Status: MVP accepted with explicit operator blockers
- Trigger: `cargo run -p daemon -- --ingest landed-cost`
- Implementation trigger validated through `cargo run -p daemon --bin daemon -- --ingest landed-cost`; `crates/daemon/Cargo.toml` now sets `default-run = "daemon"` so the shorter handoff command resolves correctly.

## Code Changes

- Added Stage 11 live-loop dispatch for `--ingest landed-cost`.
- Added Stage 11 assessment loading from `candidate_pricing_assessments` and selected `supplier_quotes`.
- Added stable `landed_cost_estimates` upsert by product candidate and selected quote.
- Added `"landed cost"` stage tracking with `scope_kind = "candidate_quote"`.
- Added bounded Stage 11 skill execution so stalled external agents become persisted blockers rather than silent skips.
- Added Rust parsers/domain validation for HMRC customs, shipment sizing, SeaRates freight, and simple FX result contracts.
- Added tests for domestic row persistence, blocked import persistence, landed-cost arithmetic, HMRC rebar parsing/canary behavior, and SeaRates parsing.

## Skill Directories

- `skills/landed-cost/hmrc-customs-classification/`
- `skills/landed-cost/shipment-sizing/`
- `skills/landed-cost/searates-freight-estimate/`
- `skills/landed-cost/simple-fx-lookup/`

## Baseline Counts

Captured before implementation:

```text
candidate_pricing_assessments=4
landed_cost_estimates=0
supplier_quotes=10
non_gbp_quotes=0
```

## Live Run Result

Final successful Stage 11 run:

```text
landed-cost live loop completed: attempted=2 completed=1 blocked=1 domestic=1 import=1 customs_skill_runs=1 sizing_skill_runs=0 searates_skill_runs=0 fx_skill_runs=0
```

Earlier in the session, two domestic rows had already persisted; the final run completed the remaining domestic row and the import blocker row.

## Post-Run Counts

```text
landed_rows=4
decision_grade=review  count=3
decision_grade=blocked count=1
orphan_quote_rows=0
orphan_candidate_rows=0
stage10_selected_without_landed_row=0
landed_rows_with_stale_quote_version=0
process_stage_heads(stage='landed cost', scope_kind='candidate_quote')=4
```

## Per-Candidate Rows

| Candidate | Source | Ship from | Mode | Grade | Supplier cost | Freight base | Duty | Remedy | Import VAT | Economic cost | Base net margin | Blockers |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| Aggregates (general) | amazon-uk | GB | domestic | review | 1540p | 0p | 0p | 0p | 0p | 1540p | 9999 | [] |
| Ely Library carpet / flooring package | amazon-uk | GB | domestic | review | 6495p | 0p | 0p | 0p | 0p | 6495p | 0 | [] |
| Library carpet / floor covering | amazon-uk | GB | domestic | review | 8093p | 0p | 0p | 0p | 0p | 8093p | 0 | [] |
| Double-glazed unit | alibaba-search | CN | unknown_import | blocked | 5160000p | 0p | 0p | 0p | 1032000p | 6192000p | 0 | `CUSTOMS_SKILL_NO_RESULT`, `CDP_CONNECT_FAILED`, `SEARATES_NO_RESULT` |

## Customs Evidence

- HMRC endpoint checked live: `https://www.trade-tariff.service.gov.uk/uk/api/commodities/7214200020`
- Endpoint returned commodity description: `Concrete reinforcement bars and rods excluding long products, such as round bars`.
- Included import measure types contained `Third country duty`, `Value added tax`, `Additional duties`, and `Additional duties (safeguard)`.
- Unit canary confirms Stage 11 does not treat `7214200000`-style parent classification as enough when import measures require the declarable rebar branch.

The live DGU import quote ran the HMRC customs skill through the Stage 11 skill runner, but the external agent did not return a usable result inside the 90 second MVP bound. Stage 11 persisted the DGU row with `CUSTOMS_SKILL_NO_RESULT` instead of skipping it.

## Shipment And SeaRates Evidence

- Shipment sizing is a separate Stage 11 skill/result contract.
- SeaRates is a separate CDP/browser skill/result contract.
- CDP preflight result during validation:

```text
curl: (7) Failed to connect to localhost port 9222 after 0 ms: Couldn't connect to server
```

Because the persistent browser was unavailable, the DGU row persisted `CDP_CONNECT_FAILED`; SeaRates was not called with fake dimensions or fake freight. The SeaRates parser test covers quoted output including `quote_expired = true`, so expired prices can be captured when the browser can reach SeaRates.

## FX Result

Current supplier quote data remained GBP-only (`non_gbp_quotes=0`). The simple FX skill and parser exist for non-GBP freight, but no live FX conversion was needed in this validation run.

## MVP Policy Choices

- Domestic quotes use zero import duty, zero import VAT, and zero domestic final-mile unless supplier evidence includes delivery cost.
- Unknown import VAT recoverability is conservative: import VAT is included in economic landed cost and cash metrics.
- SeaRates failure policy is blocker-first with zero freight, not placeholder freight.
- Quantity basis is recorded in `confidence_breakdown`; current rows used selected quote quantity or unit quote basis.
- Reruns detect missing rows, selected-quote changes, selected quote row-version drift, supplier quote row-version drift, and prior blocker rows. Full tariff/freight/FX freshness TTL policy is deferred to bead `testproject1-obep`.

## Quality Gates

```text
cargo fmt --check                                      PASS
cargo check --all-targets                              PASS
cargo clippy --all-targets -- -D warnings              PASS
cargo test -p daemon landed_cost -- --nocapture         PASS (3 tests)
cargo test -p daemon hmrc -- --nocapture                PASS (2 tests)
cargo test -p daemon searates -- --nocapture            PASS (1 test)
```

## Follow-Up Beads

- `testproject1-obep` - Implement Stage 11 tariff/freight/FX freshness TTL reruns.
- `testproject1-z7zz` - Document persistent CDP setup for Stage 11 SeaRates validation.

## Remaining Limitations

- A persistent CDP browser on port 9222 is required for live Alibaba listing inspection and SeaRates quote lookup.
- The DGU import row is intentionally blocked until CDP is available and/or the HMRC skill returns a usable classification contract.
- Stage 11 is MVP screening-grade; broker-grade customs measure interaction, TRA watchers, CBAM, Freightos, Xe, and Stage 12 are intentionally out of scope.
