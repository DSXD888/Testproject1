---
skill_id: landed-cost/shipment-sizing
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

# landed-cost/shipment-sizing

## Mission
Resolve the shipment basis required before any freight quote: units, total weight in kg, total cubic metres, package count, and LCL/FCL/container plan.

## Inputs
The daemon supplies the same candidate and selected quote context used by the HMRC skill, plus the selected supplier cost basis.

## Evidence Priority
1. Selected quote evidence fields.
2. Supplier/listing URL if present.
3. Persistent CDP browser inspection for Alibaba or marketplace listings.
4. Candidate specification text and numeric specs.
5. Quote quantity/MOQ/listing pack fields.

If CDP is required, verify the persistent browser first:

```bash
CDP_PORT=${AGENT_BROWSER_CDP_PORT:-9222}
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

Use `agent-browser --cdp ${CDP_PORT}` for all browser actions. Do not launch a fresh browser, log in, contact suppliers, or mutate external state.

## Rules
- Do not guess precision. If listing/spec evidence lacks dimensions or weight, block with `SHIPMENT_DIMENSIONS_MISSING`.
- If a quote gives square metres but no thickness/packing/unit mass, inspect source evidence or block.
- Choose LCL for genuinely small shipments, FCL when volume/weight requires containers.
- Record load-factor/container heuristics as warnings or in `container_plan.reasoning`.
- If weight exceeds practical payload before volume fills a container, split by weight.

## Output
Write the final JSON object to `result.json` in the current working directory, matching `result.schema.json`. Also keep the final response to the daemon as the same raw JSON object; do not wrap it in Markdown.
