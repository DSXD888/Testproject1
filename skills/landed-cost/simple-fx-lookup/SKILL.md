---
skill_id: landed-cost/simple-fx-lookup
skill_version: 2.0.0
interaction_profile: http_public
result_schema_version: 2.0.0
model_profile: browse_stable_low_cost
agent_cli: codex
side_effect_class: none
auth_requirements: none
requires_cdp: false
maxPages: 3
maxScrollCycles: 1
sources:
  - id: frankfurter
    url_template: "https://api.frankfurter.app/latest?from={base}&to=GBP"
    role: commercial_spot
  - id: hmrc_monthly
    url_template: "https://www.trade-tariff.service.gov.uk/exchange_rates/monthly?year={year}"
    role: customs_monthly
---

# landed-cost/simple-fx-lookup

## Mission
Return evidence-backed GBP FX snapshots for L5 landed-cost calculation: HMRC monthly customs FX for legal customs valuation and Frankfurter commercial spot FX for margin estimation.

## Inputs
- `source_currency`
- `target_currency`, always `GBP`
- optional `amount_minor`
- optional low/base/high source amounts
- optional `mode`, one of `customs`, `commercial`, or `both` (default `both`)
- optional `effective_month` (`YYYY-MM`) for HMRC monthly customs FX

## Rules
- If `source_currency` is `GBP`, return snapshots with `rate: 1.0`, `rate_bps: 10000`, `quote_currency: "GBP"`, and the original amount as GBP pence when an amount is supplied.
- For commercial spot FX, use Frankfurter's public latest endpoint:

```bash
curl -sfS "https://api.frankfurter.app/latest?from=${source_currency}&to=GBP"
```

The response must contain `rates.GBP`; missing `rates.GBP` is a blocker, not a fallback. Convert every minor-unit input by multiplying by this rate and rounding to the nearest GBP penny. Record the URL, returned date, rate, rate basis points, source role, and checked timestamp in `snapshots`.
- For customs FX, use the HMRC monthly publication:

```bash
curl -sfS "https://www.trade-tariff.service.gov.uk/exchange_rates/monthly?year=${year}"
```

Extract the row for `source_currency` and the requested or current HMRC effective month. The HMRC snapshot is legally authoritative for customs valuation. Do not substitute Frankfurter for HMRC customs FX.
- If Frankfurter is unavailable, use the last valid commercial snapshot only when the caller provides a TTL-valid cached snapshot. Otherwise return `fx_status: "blocked"` with blocker code `FX_RATE_MISSING` for the commercial basis and reduce commercial confidence in the caller.
- If HMRC monthly FX is unavailable or unparseable for the requested effective month, return `fx_status: "blocked"` with blocker code `FX_RATE_MISSING`. Do not fabricate customs FX from spot rates.
- Never call paid FX providers or inspect paid-provider API key environment variables. Xe, OANDA, CurrencyBeacon, and other commercial FX connector accounts are out of scope for L5.

## Output
Write the final JSON object to `result.json` in the current working directory, matching `result.schema.json`.

The v2 result contract uses a `snapshots` array. Existing single-rate consumers should read the `frankfurter_commercial_spot` snapshot from that array during their migration; no compatibility shim or duplicate legacy result object is emitted.

`warnings` and `blockers` must be arrays of objects with `code`, `message`, and `severity`; never emit bare string warning codes or omit severity. Keep the final response to the daemon as the same raw JSON object; do not wrap it in Markdown.
