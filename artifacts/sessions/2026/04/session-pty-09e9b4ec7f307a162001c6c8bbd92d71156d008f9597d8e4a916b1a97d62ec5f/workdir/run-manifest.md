---
manifest_version: "1.0.0"
skill_id: "landed-cost/searates-freight-estimate"
skill_version: "1.0.0"
job_id: "job-stage11-landed_cost_searates_freight-7a2061695d224bc79b0080020f6a4083"
session_id: "session-pty-09e9b4ec7f307a162001c6c8bbd92d71156d008f9597d8e4a916b1a97d62ec5f"
execution_mode: "pty"
started_at: "2026-04-30T21:08:00Z"
completed_at: "2026-04-30T21:11:50Z"
outcome_class: "quoted_expired"
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 1
agent_claimed_pages_visited: 1
agent_claimed_source_url: "https://www.searates.com/logistics-explorer/"
agent_claimed_fingerprint_state: "operator CDP Chrome session reused on port 9222; s-token present in localStorage"
agent_claimed_locale_state: "not explicitly changed"
agent_claimed_currency_state: "USD from SeaRates GraphQL response"
---

## Summary

Used the operator's existing CDP browser session on port 9222 to open SeaRates Logistics Explorer and query `https://rates.searates.com/graphql` for the Stage 11 Shanghai to London route.

The shipment payload sizes the load as LCL: 300 kg and 0.315 CBM, with `container_count: 0`. SeaRates returned three LCL candidates, all expired as of 2026-04-30. `result.json` records the cheapest expired LCL candidate, 239 USD, as 23900 minor units with `quote_expired: true` and warning `SEARATES_PRICE_EXPIRED`.

## Steps Taken

1. Confirmed CDP availability with `curl http://localhost:9222/json/version`; Chrome 147 was reachable.
2. Opened `https://www.searates.com/logistics-explorer/` through `agent-browser --cdp 9222` and confirmed the title contained Logistics Explorer.
3. Read `s-token` from page `localStorage`; token was present, so no platform-token refresh was required.
4. Tried the mandatory FCL GraphQL path first for `shippingType:FCL`, `container:ST40`, and the canonical coordinates. It returned non-expired 40 ft rates, with the lowest observed candidate at 3720 USD, shipmentId `54532878`.
5. Queried the same endpoint using the actual shipment basis: `shippingType:LCL`, `weight:300`, `weightUnit:KG`, `volume:0.315`, `volumeUnit:M3`, `includedServices:d2d`, `portFromFees:false`, `portToFees:false`, date `2026-06-14`.
6. Selected the cheapest returned LCL candidate and wrote the final contract to `result.json`.

## Decisions Made

The FCL rate was not selected because the job input explicitly states an LCL plan with `fcl_required:false`, `container_count:0`, 300 kg, and 0.315 CBM. The selected candidate is the cheapest LCL response returned for the sized shipment.

Because all LCL candidates had `general.expired:true`, the result keeps `freight_status:"quoted"` to preserve the concrete SeaRates price, while setting `quote_expired:true` and adding the required `SEARATES_PRICE_EXPIRED` warning object.

## Warnings

- `SEARATES_PRICE_EXPIRED`: SeaRates returned concrete LCL prices, but all LCL candidates were expired as of 2026-04-30.

## Evidence

- CDP check: `Chrome/147.0.7727.119`, port 9222 reachable.
- Page evidence: title `Logistics Explorer App | Freight Calculator to Get Freight Rates Today Online`, URL `https://www.searates.com/logistics-explorer/`.
- Selected LCL rate: shipmentId `54532949`, `totalPrice:239`, `totalCurrency:USD`, `totalTransitTime:36`, `validityFrom:2024-10-04`, `validityTo:2026-04-13`, `expired:true`.
- Selected route evidence: Shanghai place -> Shanghai port `CNSHA` -> Le Havre port `FRLEH` -> London place. Leg totals were 105 USD pre-carriage, 29 USD ocean freight, 105 USD on-carriage, and 0 USD final place point.
- Candidate range evidence: LCL candidates were 239 USD, 250 USD, and 281 USD; all were expired.
