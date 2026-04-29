---
skill_id: landed-cost/searates-freight-estimate
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: codex
side_effect_class: none
requires_cdp: true
cdp_default_port: 9222
maxPages: 8
maxScrollCycles: 3
---

# landed-cost/searates-freight-estimate

## Mission
Use SeaRates in the operator's persistent CDP browser to estimate sea freight for an already-sized shipment. This is quote lookup only.

## Mandatory CDP Discipline
Before navigation:

```bash
CDP_PORT=${AGENT_BROWSER_CDP_PORT:-${job_parameters.cdp_port:-9222}}
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, emit `freight_status: "blocked"` with blocker code `CDP_CONNECT_FAILED`. Do not launch a fresh browser. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

## Workflow
1. Open `https://www.searates.com/`.
2. Enter origin and destination.
3. Enter ship date.
4. Enter total weight and CBM, or FCL container type/count.
5. Search for freight estimates.
6. Capture price, currency, route, mode/container basis, transit time if shown, source URL, raw evidence text, and caveats.

If SeaRates shows an expired estimate, record the price and set `quote_expired: true` with warning `SEARATES_PRICE_EXPIRED`.

## Safety
Do not log in/out, contact carriers, book shipments, create orders, change account settings, or write server-side state.

## Output
Write the final JSON object to `result.json` in the current working directory, matching `result.schema.json`. Also keep the final response to the daemon as the same raw JSON object; do not wrap it in Markdown.
